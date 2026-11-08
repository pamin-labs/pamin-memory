//! Span vectors kept in PostgreSQL (`span_vectors`), so a topic the index
//! lacks is written from a vector already embedded from its text rather than
//! through the model again.
//!
//! Kept vectors only save passes: a lookup or a store that fails is logged and
//! the work goes on as if nothing were kept.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use memory_core::TopicId;
use memory_index::{Embedder, SpanDocument, SpanSource, VectorCodes};
use memory_store::repository;

use crate::engine::{Engine, off_the_runtime};

/// What a span's vector is embedded from, with the hash it is kept under,
/// taken once and only if asked for.
#[derive(Clone, Debug)]
pub(crate) struct Passage<'a> {
    pub(crate) text: &'a str,
    digest: OnceLock<[u8; 32]>,
}

impl<'a> Passage<'a> {
    pub(crate) fn of(text: &'a str) -> Self {
        Self {
            text,
            digest: OnceLock::new(),
        }
    }

    /// The SHA-256 of the text, so the same text finds its vector whichever
    /// topic holds it. Taken on first use: a span the index already holds is
    /// never looked up or kept.
    pub(crate) fn digest(&self) -> [u8; 32] {
        use sha2::Digest as _;
        *self
            .digest
            .get_or_init(|| sha2::Sha256::digest(self.text.as_bytes()).into())
    }
}

/// Topics written from kept vectors, each with its spans' fp16 codes in
/// order.
pub(crate) type Supplied<'k, T> = Vec<(T, Vec<&'k VectorCodes>)>;

/// How a rebuild writes the topics it could not lend
/// ([`Kept::split_for_rebuild`]).
pub(crate) struct Unlent<'k, T, M> {
    /// The model, loaded only if a topic is left to embed.
    pub(crate) model: Option<M>,
    /// The plan the supplied topics are written under.
    pub(crate) plan: Option<String>,
    pub(crate) supplied: Supplied<'k, T>,
    pub(crate) to_embed: Vec<T>,
}

/// Vectors one model revision embedded, by plan, then by passage hash.
///
/// Held as their fp16 codes, half what they widen to, and widened a batch at
/// a time as they are written ([`write_supplied`]).
#[derive(Default)]
pub(crate) struct Kept {
    pub(crate) revision: String,
    pub(crate) by_plan: BTreeMap<String, HashMap<[u8; 32], Box<VectorCodes>>>,
}

impl Kept {
    /// Splits `topics`, each given by its spans' passages in order, into those
    /// every span of which has a vector kept under `plan` of the model at
    /// `revision`, with those vectors, and those to embed. The one test of
    /// whether a topic is written from what was kept.
    ///
    /// A topic with no spans is in neither: there is nothing to write for it,
    /// so it is neither reused nor embedded.
    pub(crate) fn split<'t, T>(
        &self,
        revision: &str,
        plan: Option<&str>,
        topics: impl IntoIterator<Item = (T, &'t [Passage<'t>])>,
    ) -> (Supplied<'_, T>, Vec<T>) {
        let kept = plan
            .filter(|_| revision == self.revision)
            .and_then(|plan| self.by_plan.get(plan));
        let (mut supplied, mut to_embed) = (Vec::new(), Vec::new());
        for (topic, passages) in topics {
            if passages.is_empty() {
                continue;
            }
            match kept.filter(|kept| covers(kept, passages)) {
                Some(kept) => supplied.push((
                    topic,
                    passages
                        .iter()
                        .map(|passage| &*kept[&passage.digest()])
                        .collect(),
                )),
                None => to_embed.push(topic),
            }
        }
        (supplied, to_embed)
    }

    /// Splits the topics a rebuild could not lend ([`Kept::split`]) under the
    /// plan that applies, loading the model with `load` only if it must: once
    /// the plan that covers the most of them ([`Kept::widest`]) still leaves a
    /// topic to embed. A loaded model then decides instead, by its own
    /// revision and plan, which are what its vectors will be of.
    ///
    /// `load` takes the model's lock, so call this before taking the index's:
    /// the model before the index, as every caller takes them, or a search
    /// holding the model and wanting the index would deadlock against it.
    ///
    /// ponytail: span_vectors is per project, not per node, so the widest
    /// plan can be another node's; nodes on one kind of hardware run one plan
    /// in practice. Checking it would mean loading the model.
    pub(crate) fn split_for_rebuild<'t, T, I, M, E>(
        &self,
        topics: I,
        load: impl FnOnce() -> Result<M, E>,
    ) -> Result<Unlent<'_, T, M>, E>
    where
        I: Iterator<Item = (T, &'t [Passage<'t>])> + Clone,
        M: std::ops::Deref<Target = Embedder>,
    {
        let mut plan = self
            .widest(topics.clone().map(|(_, passages)| passages))
            .map(str::to_string);
        let (mut supplied, mut to_embed) =
            self.split(&self.revision, plan.as_deref(), topics.clone());
        let mut model = None;
        if !to_embed.is_empty() {
            let loaded = load()?;
            plan = loaded.reusable_encoding();
            (supplied, to_embed) = self.split(&loaded.revision(), plan.as_deref(), topics);
            model = Some(loaded);
        }
        Ok(Unlent {
            model,
            plan,
            supplied,
            to_embed,
        })
    }

    /// The plan whose kept vectors cover the most of `topics` whole, ties to
    /// the first by name, so a choice made with no model loaded is the same
    /// every time.
    fn widest<'t>(&self, topics: impl Iterator<Item = &'t [Passage<'t>]> + Clone) -> Option<&str> {
        // Reversed because `max_by_key` keeps the last of equals.
        self.by_plan
            .iter()
            .rev()
            .max_by_key(|(_, kept)| topics.clone().filter(|spans| covers(kept, spans)).count())
            .map(|(plan, _)| plan.as_str())
    }
}

/// Whether every one of a topic's spans, of which it has at least one, has a
/// vector in `kept`.
fn covers(kept: &HashMap<[u8; 32], Box<VectorCodes>>, passages: &[Passage<'_>]) -> bool {
    !passages.is_empty()
        && passages
            .iter()
            .all(|passage| kept.contains_key(&passage.digest()))
}

/// The hashes of every span of `topics`, to look up what is kept for them.
pub(crate) fn digests<'t>(topics: impl IntoIterator<Item = &'t [Passage<'t>]>) -> Vec<[u8; 32]> {
    topics
        .into_iter()
        .flatten()
        .map(|passage| passage.digest())
        .collect()
}

/// Writes the topics `supplied` from their kept vectors, widened back to
/// what the index stores, with `write`; `topic` names each one's topic and
/// spans.
pub(crate) fn write_supplied<'a, T, E>(
    supplied: &[(T, Vec<&VectorCodes>)],
    topic: impl Fn(&T) -> (TopicId, &'a [SpanSource<'a>]),
    write: impl FnOnce(&[SpanDocument<'_>]) -> Result<(), E>,
) -> Result<(), E> {
    // One buffer for the batch rather than one per span.
    let width = memory_index::DIMENSIONS as usize;
    let spans = supplied.iter().map(|(_, codes)| codes.len()).sum::<usize>();
    let mut vectors = vec![0.0; spans * width];
    for (codes, vector) in supplied
        .iter()
        .flat_map(|(_, codes)| codes)
        .zip(vectors.chunks_exact_mut(width))
    {
        memory_index::widen_vector(codes, vector);
    }
    let documents = span_documents(
        supplied.iter().map(|(at, _)| topic(at)),
        vectors.chunks_exact(width),
    );
    write(&documents)
}

/// What one pass embedded, as `span_vectors` keeps it: the model revision and
/// plan that produced it, and each passage's hash with its vector.
pub(crate) struct Embedded {
    pub(crate) revision: String,
    pub(crate) encoding: String,
    pub(crate) vectors: Vec<HashedVector>,
}

/// A passage's hash and its vector's fp16 bytes, as `span_vectors` keeps them.
pub(crate) type HashedVector = ([u8; 32], Vec<u8>);

impl Embedded {
    /// `None` when no plan can be shown to have produced the vectors
    /// ([`memory_index::Embedder::encoding`]): nothing could ever match them.
    pub(crate) fn of<'p>(
        revision: String,
        encoding: Option<&str>,
        passages: impl IntoIterator<Item = &'p Passage<'p>>,
        vectors: &[Vec<f32>],
    ) -> Option<Self> {
        Some(Self {
            revision,
            encoding: encoding?.to_string(),
            vectors: passages
                .into_iter()
                .zip(vectors)
                .map(|(passage, vector)| (passage.digest(), memory_index::vector_to_bytes(vector)))
                .collect(),
        })
    }
}

/// Every span of `topics`, in order, as the index writes them, with
/// `vectors`.
pub(crate) fn span_documents<'a>(
    topics: impl IntoIterator<Item = (TopicId, &'a [SpanSource<'a>])>,
    vectors: impl IntoIterator<Item = &'a [f32]>,
) -> Vec<SpanDocument<'a>> {
    topics
        .into_iter()
        .flat_map(|(topic, spans)| {
            spans
                .iter()
                .enumerate()
                .map(move |(ordinal, span)| (topic, ordinal as u32, *span))
        })
        .zip(vectors)
        .map(|((topic, ordinal, span), vector)| span.document(topic, ordinal, vector))
        .collect()
}

impl Engine {
    /// The vectors the model at `revision` embedded for the passages hashed
    /// `hashes` ([`Passage::digest`]), under `plan`, or under every plan when
    /// it is `None`. One lookup, in its own transaction, never under the
    /// model or index lock.
    pub(crate) async fn kept_vectors(
        &self,
        revision: String,
        plan: Option<&str>,
        hashes: &[[u8; 32]],
    ) -> Kept {
        let mut kept = Kept {
            revision,
            by_plan: BTreeMap::new(),
        };
        if hashes.is_empty() {
            return kept;
        }
        let rows = self
            .scoped(async |connection| {
                Ok(
                    repository::span_vectors(
                        connection,
                        self.project,
                        &kept.revision,
                        plan,
                        hashes,
                    )
                    .await?,
                )
            })
            .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(%error, "could not read kept span vectors; embedding instead");
                return kept;
            }
        };
        for row in rows {
            let Some(codes) = memory_index::vector_from_bytes(&row.vector) else {
                tracing::warn!(
                    encoding = row.encoding,
                    bytes = row.vector.len(),
                    "skipping a kept span vector of another width"
                );
                continue;
            };
            kept.by_plan
                .entry(row.encoding)
                .or_default()
                .insert(row.passage_hash, codes);
        }
        kept
    }

    /// What a rebuild that has not loaded the model can stand in for `topics`
    /// with: every plan the next load's revision kept for them
    /// ([`Kept::split_for_rebuild`]). Nothing while no revision is pinned to
    /// match.
    ///
    /// ponytail: with no model cached there is no revision to look up by, so
    /// a rebuild embeds everything even when the database kept it all.
    /// Deployments that ship the model cached never meet this; ask the loaded
    /// model's revision instead if a node ever starts without it.
    pub(crate) async fn kept_for_rebuild(&self, topics: &[&[Passage<'_>]]) -> Kept {
        let Some(revision) = off_the_runtime(|| self.models.cached_revision()) else {
            return Kept::default();
        };
        // ponytail: holds every kept vector of these topics at once, so a
        // rebuild's peak follows the project here; page it by topic if that
        // matters.
        self.kept_vectors(revision, None, &digests(topics.iter().copied()))
            .await
    }

    /// Keeps what passes embedded, so a rebuild or another node's catch-up
    /// writes it back instead of embedding it again. One insert per model
    /// revision and plan, in its own transaction after the passes: one held
    /// across them would hold a connection for as long as the model runs.
    /// What is written to the index stands without it, so a failure is
    /// logged and costs only a later pass.
    pub(crate) async fn keep_vectors(&self, embedded: &[Embedded]) {
        if embedded.is_empty() {
            return;
        }
        let mut by_plan: BTreeMap<(&str, &str), Vec<&HashedVector>> = BTreeMap::new();
        for pass in embedded {
            by_plan
                .entry((&pass.revision, &pass.encoding))
                .or_default()
                .extend(&pass.vectors);
        }
        let kept = self
            .scoped(async |connection| {
                for ((revision, encoding), vectors) in &by_plan {
                    repository::put_span_vectors(
                        connection,
                        self.project,
                        revision,
                        encoding,
                        vectors.iter().copied(),
                    )
                    .await?;
                }
                Ok(())
            })
            .await;
        if let Err(error) = kept {
            tracing::warn!(%error, "could not keep span vectors");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kept, Passage};

    /// A topic is supplied only when every span has a vector under the plan
    /// and revision asked for, in order; a topic with no spans is neither
    /// supplied nor embedded; and the widest plan is chosen, ties to the
    /// first by name.
    #[test]
    fn a_topic_is_kept_whole_under_one_plan_or_embedded() {
        let [x, y, z] = ["x", "y", "z"].map(Passage::of);
        let [one, two, three, four] =
            [1, 2, 3, 4].map(|code| Box::new([code; memory_index::DIMENSIONS as usize]));
        let kept = Kept {
            revision: "model@1".into(),
            by_plan: [
                (
                    "a".into(),
                    [(x.digest(), one.clone()), (y.digest(), two.clone())].into(),
                ),
                ("b".into(), [(x.digest(), three)].into()),
                ("c".into(), [(y.digest(), four)].into()),
            ]
            .into(),
        };
        let topics: [(u8, &[Passage<'_>]); 4] =
            [(0, &[y, x.clone()]), (1, &[x]), (2, &[z]), (3, &[])];
        let (supplied, to_embed) = kept.split("model@1", Some("a"), topics);
        assert_eq!(supplied, [(0, vec![&*two, &*one]), (1, vec![&*one])]);
        assert_eq!(to_embed, [2], "an empty topic is in neither");
        assert_eq!(kept.split("model@1", Some("b"), topics).1, [0, 2]);
        assert_eq!(
            kept.split("model@1", None, topics),
            (Vec::new(), vec![0, 1, 2])
        );
        assert_eq!(
            kept.split("model@2", Some("a"), topics),
            (Vec::new(), vec![0, 1, 2]),
            "another revision's vectors are not this model's"
        );
        let spans = topics.map(|(_, spans)| spans);
        assert_eq!(kept.widest(spans.into_iter()), Some("a"));
        assert_eq!(
            kept.widest(spans[1..].iter().copied()),
            Some("a"),
            "a and b both cover one; a is first by name"
        );
    }
}
