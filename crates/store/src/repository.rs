//! Reads and writes against the authority store.
//!
//! Every write appends. Nothing here updates a row in place except a soft
//! delete, which sets `deleted_at` and leaves the content intact.

use memory_core::{
    Derivation, EdgeKind, FilterDecision, JobKind, Project, ProjectId, SourceId, SourceKind,
    SourceSpan, SourceSpanId, SourceVersion, SourceVersionId, TombstoneReason, Topic, TopicId,
    TopicState, TopicStateId, Validity,
};
use sqlx::postgres::PgRow;
use sqlx::{PgExecutor, PgPool, Row};
use time::OffsetDateTime;

use crate::error::Result;
use crate::sql::{SqlLabel, sql_enum};

/// Version counters are `INTEGER`, which bounds a topic at two billion versions.
/// Converting through `i32` is therefore lossless in practice, and the cast is
/// kept in one place rather than scattered through each query.
fn to_sql_version(version: u32) -> i32 {
    version as i32
}

fn from_sql_version(version: i32) -> u32 {
    version as u32
}

sql_enum!(FilterDecision {
    Promoted => "promoted",
    Filtered => "filtered",
});

sql_enum!(SourceKind {
    Manual => "manual",
    File => "file",
});

sql_enum!(EdgeKind {
    Mentions => "mentions",
    Supports => "supports",
    Contradicts => "contradicts",
    Supersedes => "supersedes",
    RelatedTo => "related_to",
    PartOf => "part_of",
    DerivedFrom => "derived_from",
    SameAs => "same_as",
    DependsOn => "depends_on",
});

sql_enum!(Derivation {
    Explicit => "explicit",
    Deterministic => "deterministic",
    Model => "model",
    Imported => "imported",
});

sql_enum!(TombstoneReason {
    Closed => "closed",
    Superseded => "superseded",
    Deleted => "deleted",
});

sql_enum!(JobKind {
    SyncTopicIndex => "sync_topic_index",
    DeriveMentions => "derive_mentions",
    BackfillMentions => "backfill_mentions",
    OptimizeIndex => "optimize_index",
});

/// Returns the project with this name, creating it if it does not exist.
///
/// Reads before it writes. Every command opens with this call, and a project is
/// created once and then found forever after, so the write is the rare case.
/// Reaching it through `ON CONFLICT DO UPDATE` -- which is what a conflict
/// clause has to do to return the existing row -- made every command take a row
/// lock on the one row all of them share, and leave a dead tuple behind for
/// autovacuum. `DO NOTHING` returns nothing on conflict, so the losing side of
/// the race reads the winner's row instead.
pub async fn ensure_project(pool: &PgPool, name: &str) -> Result<Project> {
    if let Some(project) = find_project(pool, name).await? {
        return Ok(project);
    }
    let inserted = sqlx::query(include_str!("../sql/repository/ensure_project.sql"))
        .bind(ProjectId::new().0)
        .bind(name)
        .bind(OffsetDateTime::now_utc())
        .fetch_optional(pool)
        .await?;
    match inserted {
        Some(row) => Ok(project_from(&row)),
        // Another writer created it in between.
        None => find_project(pool, name)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound.into()),
    }
}

/// The project with this name, if there is one.
pub async fn find_project(pool: &PgPool, name: &str) -> Result<Option<Project>> {
    let row = sqlx::query(include_str!("../sql/repository/find_project.sql"))
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(project_from))
}

fn project_from(row: &sqlx::postgres::PgRow) -> Project {
    Project {
        id: row.get::<uuid::Uuid, _>("id").into(),
        name: row.get("name"),
        created_at: row.get("created_at"),
    }
}

/// What [`purge_project`] deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgedProject {
    pub id: ProjectId,
    pub topics: u64,
    pub sources: u64,
    /// The paths it kept in sync, for the caller to stop watching.
    pub synced: Vec<String>,
}

/// Deletes this project and every row it owns, for good.
///
/// Unlike a retraction, which closes a validity interval and keeps the
/// history, this is for privacy requests: nothing of the project is left to
/// query. Every table references `projects` with `ON DELETE CASCADE`, so one
/// statement removes it all; the counts come from the same statement's
/// snapshot, taken before the delete. `None` when there is no such project.
///
/// Run it in the project's [`scoped`](crate::Database::scoped) transaction:
/// the counts read guarded tables, which show an owner nothing (V21).
pub async fn purge_project(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Option<PurgedProject>> {
    let row: Option<(uuid::Uuid, i64, i64, Vec<String>)> =
        sqlx::query_as(include_str!("../sql/repository/purge_project.sql"))
            .bind(project.0)
            .fetch_optional(executor)
            .await?;
    Ok(row.map(|(id, topics, sources, synced)| PurgedProject {
        id: id.into(),
        topics: topics as u64,
        sources: sources as u64,
        synced,
    }))
}

/// Returns the source with this locator, creating it if it does not exist,
/// and holds it locked until the transaction ends.
///
/// Re-ingesting the same locator appends a version to the existing source
/// rather than forking a second one, which is what keeps a file's history in a
/// single chain.
///
/// The lock is what [`append_evidence`] and [`append_promoted`] number under, taken here
/// because this is the statement that finds the row: locking it again by id
/// was a second round trip for the same row. A row this call inserts is
/// already held by the inserting transaction, so both paths leave it locked.
/// Outside a transaction the lock lasts one statement and means nothing.
///
/// The lookup and the insert are one statement. The insert runs only when the
/// lookup found nothing, which is a question the statement can ask itself, so
/// a new source -- every first write to a topic -- no longer pays a round trip
/// to learn that it has to ask a second question.
pub async fn ensure_source(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    kind: SourceKind,
    locator: &str,
) -> Result<SourceId> {
    Ok(ensure_source_created(connection, project, kind, locator)
        .await?
        .0)
}

/// [`ensure_source`], saying whether this call created the source -- which is
/// the first time anything was filed under `locator`.
pub async fn ensure_source_created(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    kind: SourceKind,
    locator: &str,
) -> Result<(SourceId, bool)> {
    const FIND: &str = include_str!("../sql/repository/lock_existing_source.sql");

    let row = sqlx::query(include_str!("../sql/repository/ensure_source.sql"))
        .bind(project.0)
        .bind(locator)
        .bind(SourceId::new().0)
        .bind(kind.label())
        .bind(OffsetDateTime::now_utc())
        .fetch_optional(&mut *connection)
        .await?;

    let row = match row {
        Some(row) => row,
        // Another writer created it after this statement's snapshot was taken:
        // the insert waited for that writer and then did nothing, and the
        // lookup had already looked. A statement of its own sees the row.
        None => {
            sqlx::query(FIND)
                .bind(project.0)
                .bind(locator)
                .fetch_one(&mut *connection)
                .await?
        }
    };

    Ok((row.get::<uuid::Uuid, _>("id").into(), row.get("created")))
}

/// Evidence as the write path records it: the content, the filter's verdict
/// on it, and the language detected over it.
pub struct Evidence<'a> {
    pub content: &'a str,
    pub content_hash: &'a str,
    pub decision: FilterDecision,
    pub reason: &'a str,
    pub language: Option<&'a str>,
    pub language_confidence: Option<f32>,
}

/// Appends evidence and the span that covers all of it, in one statement,
/// along with the filter's verdict on it.
///
/// What a write the filter holds records. The verdict rides on a row that
/// exists either way: the filter decides whether content reaches the retrieval
/// surface, never whether it is kept.
///
/// **Call it after [`ensure_source`], in the same transaction.** The version
/// number is read and written under the lock that call takes on the source
/// row. Two agents writing to one source otherwise both read the same maximum
/// and both claim the version after it, and only one of the two rows survives
/// the uniqueness constraint. Losing the other is losing evidence, which is
/// the one thing this store promises never to do. The lock has to be taken in
/// a statement before this one: a statement reads with the snapshot it began
/// with, so one that waited for the lock inside itself would still number from
/// what it saw before the wait.
pub async fn append_evidence(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    source: SourceId,
    evidence: &Evidence<'_>,
) -> Result<(SourceVersion, SourceSpan)> {
    let (version, span, _) = insert_evidence(connection, project, source, evidence, None).await?;
    Ok((version, span))
}

/// What a promoted write records beyond its evidence: the topic it is
/// promoted into, when and for how long it holds, and the work it owes.
pub struct Promotion<'a> {
    /// Locked by an earlier statement of the same transaction, which is what
    /// holding one says.
    pub topic: &'a LockedTopic,
    pub observed_at: OffsetDateTime,
    pub validity: Validity,
    /// The cascade's work, queued the way [`crate::jobs::enqueue_all`] queues
    /// it, against the topic.
    pub owed: &'a [JobKind],
}

/// Appends evidence, the span over all of it, the topic's state cut from that
/// span, the topic's pointer to it, and the work the cascade owes it -- all in
/// one statement.
///
/// Everything a promoted write records once its locks are held. They were
/// four statements, and each needed nothing from the one before it that the
/// caller could not choose first: the version's id, the span's id, the state's
/// id are all picked here, and the predecessor the state supersedes was read
/// by the statement that locked the topic. A data-modifying `WITH` runs
/// whether or not the outer query reads it, and every foreign key between
/// these rows is checked at the end of the statement, by which point all of
/// them exist.
///
/// **Call it after [`ensure_source`] and after [`lock_topic`] or
/// [`create_topic_named`], in the same transaction.** Both numbers -- the
/// evidence's version and the state's -- are read and written under those
/// locks, and a lock only protects a number read by a statement that began
/// after it was granted.
pub async fn append_promoted(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    source: SourceId,
    evidence: &Evidence<'_>,
    promotion: &Promotion<'_>,
) -> Result<(SourceVersion, SourceSpan, TopicState)> {
    let (version, span, state) =
        insert_evidence(connection, project, source, evidence, Some(promotion)).await?;
    Ok((version, span, state.expect("asked for the state")))
}

/// The version, the span over all of it, and with a `promotion` the state cut
/// from that span, written by one statement.
async fn insert_evidence(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    source: SourceId,
    evidence: &Evidence<'_>,
    promotion: Option<&Promotion<'_>>,
) -> Result<(SourceVersion, SourceSpan, Option<TopicState>)> {
    const WITH_SPAN: &str = include_str!("../sql/repository/insert_evidence.sql");
    const PROMOTED: &str = include_str!("../sql/repository/insert_promoted_evidence.sql");

    let span_id = SourceSpanId::new();
    let byte_end = evidence.content.len() as u32;
    let query = sqlx::query(match promotion {
        None => WITH_SPAN,
        Some(_) => PROMOTED,
    })
    .bind(SourceVersionId::new().0)
    .bind(project.0)
    .bind(source.0)
    .bind(evidence.content)
    .bind(evidence.content_hash)
    .bind(evidence.decision.label())
    .bind(evidence.reason)
    .bind(OffsetDateTime::now_utc())
    .bind(span_id.0)
    .bind(byte_end as i32)
    .bind(evidence.language)
    .bind(evidence.language_confidence);
    let query = match promotion {
        None => query,
        Some(promotion) => crate::jobs::Queued::of(promotion.owed).bind(
            query
                .bind(TopicStateId::new().0)
                .bind(promotion.topic.topic.id.0)
                .bind(promotion.observed_at)
                .bind(promotion.topic.current.map(|id| id.0))
                .bind(promotion.validity.from)
                .bind(promotion.validity.to),
        ),
    };
    let row = query.fetch_one(connection).await?;

    let version = SourceVersion {
        id: row.get::<uuid::Uuid, _>("id").into(),
        project_id: project,
        source_id: source,
        version: from_sql_version(row.get("version")),
        content: evidence.content.to_string(),
        content_hash: evidence.content_hash.to_string(),
        filter_decision: evidence.decision,
        filter_reason: evidence.reason.to_string(),
        recorded_at: row.get("recorded_at"),
    };
    let span = SourceSpan {
        id: span_id,
        project_id: project,
        source_version_id: version.id,
        byte_start: 0,
        byte_end,
        detected_language: evidence.language.map(str::to_string),
        language_confidence: evidence.language_confidence,
    };
    let state = promotion.map(|promotion| TopicState {
        id: row.get::<uuid::Uuid, _>("state_id").into(),
        project_id: project,
        topic_id: promotion.topic.topic.id,
        version: from_sql_version(row.get("state_version")),
        // The span is the whole of the evidence, so the state says all of it.
        content: evidence.content.to_string(),
        source_span_id: span_id,
        language: evidence.language.map(str::to_string),
        observed_at: promotion.observed_at,
        recorded_at: version.recorded_at,
        validity: promotion.validity,
        supersedes: promotion.topic.current,
        deleted_at: None,
    });
    Ok((version, span, state))
}

/// A topic held locked by the transaction that found it, with the state it
/// pointed at when the lock was granted.
///
/// What [`append_promoted`] requires: holding one is the proof that the lock
/// the append numbers under was taken by an earlier statement.
pub struct LockedTopic {
    pub topic: Topic,
    /// The newest surviving state, which the next one supersedes.
    pub current: Option<TopicStateId>,
}

const LOCK_TOPIC: &str = include_str!("../sql/repository/lock_topic_by_name.sql");

/// Finds a topic by name and locks it, reading its current state under the
/// lock. `None` when no topic has the name, and then nothing is locked.
///
/// The write path found a topic by name and then locked it by id, and the
/// second statement needed nothing from the first but the id. Asked together,
/// the lock is on the row the name found and the pointer is read under it: a
/// statement that waits for a row lock reads the row as the holder left it, so
/// this sees the pointer a concurrent append just moved, not the one before
/// it. Every path that changes which states survive moves the pointer under
/// this same lock ([`append_promoted`] and [`soft_delete_topic_state`]).
pub async fn lock_topic(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    name: &str,
) -> Result<Option<LockedTopic>> {
    let row = sqlx::query(LOCK_TOPIC)
        .bind(project.0)
        .bind(name)
        .fetch_optional(connection)
        .await?;
    Ok(row.map(|row| locked_topic(project, &row)))
}

/// Creates a topic, files its name in the name index, and returns it locked --
/// or returns, locked, the topic another writer created first.
///
/// For a caller that has just asked [`lock_topic`] and found nothing. A row
/// this inserts is held by the inserting transaction, so it is locked without
/// asking again. The name row goes in by the same statement: a topic that
/// exists and is missing from the name index is a topic no memory will ever
/// derive an edge to, and it was a round trip of its own that needed only the
/// id this chose. `key` and `tokens` are the name as the segmenter reads it,
/// which is why the caller computes them.
///
/// When another writer created the topic in between, the insert waits for it
/// and does nothing, the name row with it -- that writer filed its own -- and
/// the topic is found and locked by a statement of its own.
pub async fn create_topic_named(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    name: &str,
    key: &str,
    tokens: usize,
) -> Result<LockedTopic> {
    create_topic(connection, project, name, Some((key, tokens))).await
}

/// Files the name of the topic called `name` in the name index, if the topic
/// exists and is not filed there yet.
///
/// For a topic `pamin-memory ingest` created, which is left out of the index, when
/// somebody writes it by hand: from then on it is theirs too, and a name
/// somebody wrote is one a memory may name.
///
/// Returns the topic when this call filed it.
pub async fn name_topic(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    name: &str,
    key: &str,
    tokens: usize,
) -> Result<Option<TopicId>> {
    let inserted: Option<(uuid::Uuid,)> =
        sqlx::query_as(include_str!("../sql/repository/name_topic.sql"))
            .bind(project.0)
            .bind(name)
            .bind(key)
            .bind(i16::try_from(tokens).unwrap_or(i16::MAX))
            .fetch_optional(executor)
            .await?;
    Ok(inserted.map(|(topic,)| topic.into()))
}

/// [`create_topic_named`] for a topic no memory is expected to name: one cut
/// out of a file, whose name is its path and headings.
///
/// Left out of the name index rather than filed there, because a name in it is
/// a cost every later write pays and not only its own: mention derivation
/// compares every run of a memory's tokens up to the widest name in the
/// project, so one name of twenty tokens makes every memory written afterwards
/// build four times the runs one of five did. A path no prose quotes in full
/// buys nothing for that.
pub async fn create_topic_unnamed(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    name: &str,
) -> Result<LockedTopic> {
    create_topic(connection, project, name, None).await
}

async fn create_topic(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    name: &str,
    named: Option<(&str, usize)>,
) -> Result<LockedTopic> {
    let inserted = sqlx::query(include_str!("../sql/repository/create_topic.sql"))
        .bind(TopicId::new().0)
        .bind(project.0)
        .bind(name)
        .bind(OffsetDateTime::now_utc())
        .bind(named.map(|(key, _)| key))
        .bind(named.map(|(_, tokens)| tokens as i16))
        .fetch_optional(&mut *connection)
        .await?;

    let row = match inserted {
        Some(row) => row,
        None => {
            sqlx::query(LOCK_TOPIC)
                .bind(project.0)
                .bind(name)
                .fetch_one(&mut *connection)
                .await?
        }
    };
    Ok(locked_topic(project, &row))
}

fn locked_topic(project: ProjectId, row: &PgRow) -> LockedTopic {
    LockedTopic {
        topic: Topic {
            id: row.get::<uuid::Uuid, _>("id").into(),
            project_id: project,
            name: row.get("name"),
            path: row.get("path"),
            created_at: row.get("created_at"),
        },
        current: row
            .get::<Option<uuid::Uuid>, _>("current_state_id")
            .map(TopicStateId::from),
    }
}

/// Sets the topic's current state, or clears it when nothing survives.
///
/// Both callers already hold the topic row locked, which is what keeps this a
/// write rather than a read-modify-write.
async fn point_at(
    connection: &mut sqlx::PgConnection,
    topic: TopicId,
    state: Option<&TopicState>,
) -> Result<()> {
    sqlx::query(include_str!("../sql/repository/point_at.sql"))
        .bind(topic.0)
        .bind(state.map(|state| state.id.0))
        .bind(state.map(|state| to_sql_version(state.version)))
        .execute(connection)
        .await?;

    Ok(())
}

fn row_to_topic_state(row: &PgRow) -> TopicState {
    TopicState {
        id: row.get::<uuid::Uuid, _>("id").into(),
        project_id: row.get::<uuid::Uuid, _>("project_id").into(),
        topic_id: row.get::<uuid::Uuid, _>("topic_id").into(),
        version: from_sql_version(row.get("version")),
        content: content_of_span(row),
        source_span_id: row.get::<uuid::Uuid, _>("source_span_id").into(),
        language: row.get("detected_language"),
        observed_at: row.get("observed_at"),
        recorded_at: row.get("recorded_at"),
        validity: Validity::new(row.get("valid_from"), row.get("valid_to")),
        supersedes: row
            .get::<Option<uuid::Uuid>, _>("supersedes")
            .map(Into::into),
        deleted_at: row.get("deleted_at"),
    }
}

/// The text a span covers: `byte_start..byte_end` of its evidence.
///
/// Byte offsets, as `SourceSpan` defines them, which is why the cut is made
/// here rather than with SQL's `substring` -- that counts characters, and the
/// two part company at the first character outside ASCII.
fn span_text(evidence: &str, byte_start: u32, byte_end: u32) -> &str {
    &evidence[byte_start as usize..byte_end as usize]
}

/// A state's content, cut from the evidence joined through its source span.
///
/// Every span the write path records covers its evidence whole, so the usual
/// case hands the string over rather than copying it.
fn content_of_span(row: &PgRow) -> String {
    let evidence: String = row.get("evidence");
    let (start, end) = (
        row.get::<i32, _>("byte_start") as u32,
        row.get::<i32, _>("byte_end") as u32,
    );
    if start == 0 && end as usize == evidence.len() {
        evidence
    } else {
        span_text(&evidence, start, end).to_string()
    }
}

/// Returns a topic's undeleted version numbers, oldest first.
///
/// This is what version resolution runs against, so soft-deleted versions are
/// excluded here rather than filtered afterwards.
pub async fn topic_versions(executor: impl PgExecutor<'_>, topic: TopicId) -> Result<Vec<u32>> {
    let versions: Vec<(i32,)> =
        sqlx::query_as(include_str!("../sql/repository/topic_versions.sql"))
            .bind(topic.0)
            .fetch_all(executor)
            .await?;

    Ok(versions
        .into_iter()
        .map(|(version,)| from_sql_version(version))
        .collect())
}

/// Loads one version of a topic.
///
/// Takes the project because the key it reads is `(project_id, topic_id,
/// version)`, and without its first column that is a walk of the whole index
/// -- every project's states -- to find one row.
pub async fn topic_state(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topic: TopicId,
    version: u32,
) -> Result<Option<TopicState>> {
    let row = sqlx::query(include_str!("../sql/repository/topic_state.sql"))
        .bind(project.0)
        .bind(topic.0)
        .bind(to_sql_version(version))
        .fetch_optional(executor)
        .await?;

    Ok(row.as_ref().map(row_to_topic_state))
}

/// Every topic's current state, one row per topic.
///
/// What a rebuild indexes. The projection holds one document per topic, so
/// feeding it every live state would write a topic's fourteen versions onto one
/// key and leave whichever the scan reached last -- which is not the same thing
/// as the one the topic stands for.
pub async fn all_current_topic_states(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Vec<TopicState>> {
    let rows = sqlx::query(include_str!(
        "../sql/repository/all_current_topic_states.sql"
    ))
    .bind(project.0)
    .fetch_all(executor)
    .await?;

    Ok(rows.iter().map(row_to_topic_state).collect())
}

/// The topics [`all_current_topic_states`] would return, without their states.
///
/// For a caller that needs to know which topics the projection should hold
/// and not what they say -- a reshape copies each document from the index it
/// already has, so reading every topic's content out of the ledger to learn
/// its identifier would load the whole project to use none of it. Filtered the
/// same way, so the two can never disagree about which topics are live.
pub async fn current_topic_ids(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Vec<TopicId>> {
    let ids: Vec<uuid::Uuid> =
        sqlx::query_scalar(include_str!("../sql/repository/current_topic_ids.sql"))
            .bind(project.0)
            .fetch_all(executor)
            .await?;

    Ok(ids.into_iter().map(TopicId::from).collect())
}

/// Loads the states these topics currently resolve to.
///
/// Through the pointer on `topics`, so this is a primary key lookup per topic
/// rather than a search for each one's newest surviving version. The graph
/// channel walks topic identities and can only return states, so every
/// neighbour it finds comes through here.
///
/// A topic whose every state has been soft deleted resolves to nothing and is
/// simply absent from the result.
pub async fn current_states_of(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topics: &[TopicId],
) -> Result<Vec<TopicState>> {
    Ok(current_states_named(executor, project, topics)
        .await?
        .into_iter()
        .map(|(_, state)| state)
        .collect())
}

/// [`current_states_of`], with each topic's name beside its state.
///
/// The statement already joins `topics` to follow the pointer, so the name is
/// a column away. The search path needs both -- the state to rank and the name
/// to show -- and asked for the name in a second round trip over the same
/// rows.
pub async fn current_states_named(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topics: &[TopicId],
) -> Result<Vec<(String, TopicState)>> {
    if topics.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<uuid::Uuid> = topics.iter().map(|topic| topic.0).collect();
    let rows = sqlx::query(include_str!("../sql/repository/current_states_named.sql"))
        .bind(project.0)
        .bind(&ids)
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| (row.get("topic_name"), row_to_topic_state(row)))
        .collect())
}

/// What one topic currently says, found by name.
///
/// Through the pointer on `topics`, like [`current_states_of`], and for the
/// same reason: the alternative is a search for the topic's newest surviving
/// version. The write path used to do exactly that -- read *every* version of
/// the topic, ordered, into memory, and take the last one -- to answer a
/// question the pointer answers directly. That is three round trips and a scan
/// that grows with the topic's edit history, on every write, to compare one
/// string.
///
/// Only the content, because that is the whole question the caller has: is what
/// is being written what the topic already says.
///
/// A topic that does not exist and a topic whose every state has been soft
/// deleted both resolve to nothing, and the caller treats them the same -- in
/// both cases there is nothing for the new content to be identical to.
pub async fn current_content(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    name: &str,
) -> Result<Option<String>> {
    let row = sqlx::query(include_str!("../sql/repository/current_content.sql"))
        .bind(project.0)
        .bind(name)
        .fetch_optional(executor)
        .await?;

    Ok(row.as_ref().map(content_of_span))
}

/// Names these topics, and says which state each currently resolves to.
///
/// One lookup for the two things the search path needs about a topic once it
/// has a result from it: what to call it, and whether the state in hand is the
/// one the topic stands for now.
pub async fn topics_by_id(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topics: &[TopicId],
) -> Result<Vec<(TopicId, String, Option<TopicStateId>)>> {
    if topics.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<uuid::Uuid> = topics.iter().map(|topic| topic.0).collect();
    let rows = sqlx::query(include_str!("../sql/repository/topics_by_id.sql"))
        .bind(project.0)
        .bind(&ids)
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| {
            (
                row.get::<uuid::Uuid, _>("id").into(),
                row.get("name"),
                row.get::<Option<uuid::Uuid>, _>("current_state_id")
                    .map(Into::into),
            )
        })
        .collect())
}

/// Soft deletes one version of a topic.
///
/// The row and its content stay: deletion removes a state from the default
/// retrieval surface, not from the ledger. If the deleted state was current,
/// the previous surviving version becomes current -- and since that is now a
/// stored pointer rather than a computed one, moving it is part of the same
/// transaction, under the same lock the append path takes.
pub async fn soft_delete_topic_state(
    connection: &mut sqlx::PgConnection,
    topic: TopicId,
    version: u32,
) -> Result<bool> {
    sqlx::query(include_str!("../sql/repository/lock_topic_by_id.sql"))
        .bind(topic.0)
        .execute(&mut *connection)
        .await?;

    let affected = sqlx::query(include_str!(
        "../sql/repository/soft_delete_topic_state.sql"
    ))
    .bind(topic.0)
    .bind(to_sql_version(version))
    .bind(OffsetDateTime::now_utc())
    .execute(&mut *connection)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(false);
    }

    // Recomputed rather than stepped back to the predecessor: the deleted state
    // need not have been the current one, and the version before it need not
    // have survived either. This asks the ledger the same question the pointer
    // is an answer to.
    let surviving = sqlx::query(include_str!("../sql/repository/newest_surviving_state.sql"))
        .bind(topic.0)
        .fetch_optional(&mut *connection)
        .await?
        .as_ref()
        .map(row_to_topic_state);

    point_at(connection, topic, surviving.as_ref()).await?;

    Ok(true)
}

/// Soft deletes every surviving version of a topic, so that it stands for
/// nothing: off the retrieval surface, its pointer cleared, and the work that
/// removes its document from the projection and closes the edges its content
/// derived owed in the same transaction. Returns whether anything survived to
/// delete.
///
/// What [`soft_delete_topic_state`] does to one version, done to all of them --
/// for a memory whose source no longer says anything at all, which is what a
/// section of a file becomes when the file stops having it. The rows and their
/// evidence stay, so the history reads as it did; a later write to the topic
/// appends a version as any write does, and brings it back.
pub async fn retire_topic(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    topic: TopicId,
) -> Result<bool> {
    sqlx::query(include_str!("../sql/repository/lock_topic_by_id.sql"))
        .bind(topic.0)
        .execute(&mut *connection)
        .await?;

    let affected = sqlx::query(include_str!("../sql/repository/retire_topic_states.sql"))
        .bind(topic.0)
        .bind(OffsetDateTime::now_utc())
        .execute(&mut *connection)
        .await?;

    if affected.rows_affected() == 0 {
        return Ok(false);
    }

    point_at(connection, topic, None).await?;
    crate::jobs::enqueue_all(
        &mut *connection,
        project,
        &[JobKind::SyncTopicIndex, JobKind::DeriveMentions],
        Some(topic.0),
    )
    .await?;

    Ok(true)
}

/// Recomputes every topic's current-state pointer from the ledger.
///
/// Returns how many topics were pointing somewhere else. Nothing should be, and
/// the two writers that move the pointer both do it under the topic's lock in
/// the transaction that changed the ledger. But a stored pointer is derived
/// data, and derived data needs a route back: without one, a third write path
/// that forgets shows up as a search returning content the topic no longer has,
/// with nothing to say so.
///
/// This is that route, and `reindex` is where it belongs -- the command whose
/// whole promise is that everything outside the ledger can be rebuilt from it.
pub async fn repair_current_state_pointers(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<u64> {
    let repaired = sqlx::query(include_str!(
        "../sql/repository/repair_current_state_pointers.sql"
    ))
    .bind(project.0)
    .execute(executor)
    .await?;

    Ok(repaired.rows_affected())
}

/// Whether any evidence is filed under `locator`.
pub async fn source_at(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    locator: &str,
) -> Result<bool> {
    let found: Option<(i32,)> = sqlx::query_as(include_str!("../sql/repository/source_at.sql"))
        .bind(project.0)
        .bind(locator)
        .fetch_optional(executor)
        .await?;
    Ok(found.is_some())
}

/// The content hash of the newest evidence filed under `locator`, if any.
///
/// For a caller that has to know whether it is about to record what is
/// already there, and has the locator rather than the source's id.
pub async fn latest_hash_at(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    locator: &str,
) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as(include_str!("../sql/repository/latest_hash_at.sql"))
            .bind(project.0)
            .bind(locator)
            .fetch_optional(executor)
            .await?;

    Ok(row.map(|(hash,)| hash))
}

/// Loads the newest evidence version for a source.
///
/// Reads back the filter verdict, which is how a caller confirms that filtered
/// content was still stored rather than discarded.
///
/// Takes the project for the reason [`topic_state`] does: the key it reads,
/// `(project_id, source_id, version)`, leads with it.
pub async fn latest_source_version(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    source: SourceId,
) -> Result<Option<SourceVersion>> {
    let row = sqlx::query(include_str!("../sql/repository/latest_source_version.sql"))
        .bind(project.0)
        .bind(source.0)
        .fetch_optional(executor)
        .await?;

    Ok(row.map(|row| SourceVersion {
        id: row.get::<uuid::Uuid, _>("id").into(),
        project_id: row.get::<uuid::Uuid, _>("project_id").into(),
        source_id: row.get::<uuid::Uuid, _>("source_id").into(),
        version: from_sql_version(row.get("version")),
        content: row.get("content"),
        content_hash: row.get("content_hash"),
        filter_decision: FilterDecision::from_label(row.get("filter_decision"))
            // The column's CHECK constraint admits nothing else, so this
            // fallback stands only so a corrupted row degrades to the
            // conservative reading rather than aborting the command.
            .unwrap_or(FilterDecision::Promoted),
        filter_reason: row.get("filter_reason"),
        recorded_at: row.get("recorded_at"),
    }))
}

/// One piece of evidence matching a literal search, with where it came from.
#[derive(Clone, Debug)]
pub struct EvidenceMatch {
    pub source_version: SourceVersion,
    pub locator: String,
    /// Byte offset of the first occurrence, for rendering context around it.
    pub offset: usize,
}

/// Finds evidence containing `needle`, verbatim.
///
/// Searches `source_versions`, which is the authority: it holds every version
/// ever written, in the language it arrived in, **including content the sensory
/// filter held**. That content never enters the projection index, so this is
/// the only way to reach it — and reaching it is the point. A filter mistake
/// has to stay recoverable, and a recovery route nobody can take is not one.
///
/// `position` rather than a regular expression or a similarity operator.
/// PostgreSQL's regex operators are outside the portable subset, and a literal
/// match is what an exact-string question actually asks for. Nothing here
/// ranks: this is the primitive an agent reaches for when it does not want a
/// ranking model in the path.
pub async fn grep_evidence(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    needle: &str,
    case_sensitive: bool,
    limit: u32,
) -> Result<Vec<EvidenceMatch>> {
    // Folding case in SQL keeps the match and the offset consistent: computing
    // one here and the other in Rust would drift on any multi-byte casing rule.
    //
    // Compute the match once so selection and filtering share the same offset.
    let sql = if case_sensitive {
        include_str!("../sql/repository/grep_evidence_case_sensitive.sql")
    } else {
        include_str!("../sql/repository/grep_evidence_case_insensitive.sql")
    };
    let rows = sqlx::query(sql)
        .bind(project.0)
        .bind(needle)
        .bind(i64::from(limit))
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let content: String = row.get("content");
            let offset = byte_offset(&content, row.get::<i32, _>("match_position") as usize);
            EvidenceMatch {
                source_version: SourceVersion {
                    id: row.get::<uuid::Uuid, _>("id").into(),
                    project_id: row.get::<uuid::Uuid, _>("project_id").into(),
                    source_id: row.get::<uuid::Uuid, _>("source_id").into(),
                    version: from_sql_version(row.get("version")),
                    content,
                    content_hash: row.get("content_hash"),
                    filter_decision: FilterDecision::from_label(row.get("filter_decision"))
                        .unwrap_or(FilterDecision::Promoted),
                    filter_reason: row.get("filter_reason"),
                    recorded_at: row.get("recorded_at"),
                },
                locator: row.get("locator"),
                offset,
            }
        })
        .collect())
}

/// Where the match SQL's `position` found starts, in bytes of `content`.
///
/// `position` answers in characters, counting from one, and an offset here is
/// bytes -- the unit `span_text` cuts in and every caller slices with. The two
/// agree only while everything before the match is ASCII. Counting the
/// characters again in Rust is exact because the cluster is initialised UTF-8,
/// where PostgreSQL's character is a code point and so is a Rust `char`. It
/// holds for the folded search too: under the default libc provider `lower`
/// maps one character to one, so a position in the folded text is the same
/// position in the original.
fn byte_offset(content: &str, position: usize) -> usize {
    content
        .char_indices()
        .nth(position.saturating_sub(1))
        .map_or(content.len(), |(byte, _)| byte)
}

/// Records how many topics' names tokenize, in one statement.
///
/// The key is computed by the caller because tokenizing is the segmenter's
/// job and the segmenter lives above this layer. A write files its one name
/// with the topic, in [`create_topic_named`]. This is what a rebuild uses,
/// because a rebuild records all of them -- and doing that a row at a time is
/// one round trip per topic, which on the corpora this project measures is
/// thirteen thousand of them for a table with no more rows than that.
///
/// `unnest` over three arrays, which is the same shape [`topics_named_by`] and
/// `graph::live_versions_of` already use for their reads. A conflict updates
/// the row, so a rebuild over a table that already has these rows rewrites
/// them rather than failing -- which is what a rebuild is.
pub async fn record_topic_names(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    names: &[(TopicId, String, usize)],
) -> Result<()> {
    if names.is_empty() {
        return Ok(());
    }

    let topics: Vec<uuid::Uuid> = names.iter().map(|(topic, _, _)| topic.0).collect();
    let keys: Vec<String> = names.iter().map(|(_, key, _)| key.clone()).collect();
    // `i16` because that is the column, and a name with more tokens than a
    // `smallint` holds is not a name.
    let counts: Vec<i16> = names
        .iter()
        .map(|(_, _, tokens)| i16::try_from(*tokens).unwrap_or(i16::MAX))
        .collect();

    sqlx::query(include_str!("../sql/repository/record_topic_names.sql"))
        .bind(project.0)
        .bind(&topics)
        .bind(&keys)
        .bind(&counts)
        .execute(executor)
        .await?;

    Ok(())
}

/// How many tokens the longest topic name in this project has.
///
/// Bounds the lookup: a run of tokens wider than the widest name cannot be a
/// name, so there is no point asking about it. Zero when the project has no
/// topics, which means there is nothing to ask about at all.
pub async fn widest_topic_name(executor: impl PgExecutor<'_>, project: ProjectId) -> Result<usize> {
    let row: (Option<i16>,) =
        sqlx::query_as(include_str!("../sql/repository/widest_topic_name.sql"))
            .bind(project.0)
            .fetch_one(executor)
            .await?;

    Ok(row.0.unwrap_or(0).max(0) as usize)
}

/// The topics whose names appear among these token runs.
///
/// The runs are every window of the text being examined, at every width up to
/// [`widest_topic_name`]. A name matches only as a contiguous run, so equality
/// against the stored key is the whole test -- there is no candidate set to
/// re-check afterwards.
pub async fn topics_named_by(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    runs: &[String],
) -> Result<Vec<TopicId>> {
    Ok(names_matching(executor, project, runs)
        .await?
        .into_iter()
        .map(|(_, topic)| topic)
        .collect())
}

/// [`topics_named_by`], with the run each topic's name matched.
///
/// For a caller asking on behalf of several texts at once: each text's runs
/// go into one question, and the run beside each answer is what says which
/// text it belongs to.
pub async fn names_matching(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    runs: &[String],
) -> Result<Vec<(String, TopicId)>> {
    if runs.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query(include_str!("../sql/repository/names_matching.sql"))
        .bind(project.0)
        .bind(runs)
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| {
            (
                row.get("name_key"),
                row.get::<uuid::Uuid, _>("topic_id").into(),
            )
        })
        .collect())
}

/// Every topic in a project that belongs in the name index: all of them but
/// the sections `pamin-memory ingest` cut out of files, which
/// [`create_topic_unnamed`] keeps out of it.
///
/// Told apart by their sources rather than their name. A section's evidence is
/// filed under `file:{name}` and a manual memory's under `manual:{name}`, and
/// the unique key on `(project_id, locator)` answers both for each topic,
/// where a rule about what a name looks like would also catch a topic someone
/// wrote by hand. A topic with both was written by hand as well as ingested,
/// and keeps its name.
pub async fn topics_to_name(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Vec<Topic>> {
    let rows = sqlx::query(include_str!("../sql/repository/topics_to_name.sql"))
        .bind(project.0)
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| Topic {
            id: row.get::<uuid::Uuid, _>("id").into(),
            project_id: project,
            name: row.get("name"),
            path: row.get("path"),
            created_at: row.get("created_at"),
        })
        .collect())
}

/// A topic `pamin-memory ingest` has put on the retrieval surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTopic {
    pub name: String,
    /// What it says now, which an ingest that retires it checks is still what
    /// it says when it gets there.
    pub current: TopicStateId,
    /// The directory it was last ingested from.
    pub root: Option<String>,
}

/// Every topic `pamin-memory ingest` has put on the retrieval surface in this project.
///
/// One read for a whole ingest, rather than one per file: an ingest compares
/// each file's sections against what an earlier ingest left under that file,
/// and asking once per file would scan the project's sources once per file.
pub async fn file_topic_names(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Vec<FileTopic>> {
    let rows: Vec<(String, uuid::Uuid, Option<String>)> =
        sqlx::query_as(include_str!("../sql/repository/file_topic_names.sql"))
            .bind(project.0)
            .fetch_all(executor)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(name, current, root)| FileTopic {
            name,
            current: current.into(),
            root,
        })
        .collect())
}

/// [`file_topic_names`] for the files an ingest is looking at, rather than
/// every file the project has: the topics whose name starts with one of
/// `prefixes` -- a file's name, or a directory's followed by `/`.
///
/// A prefix is a match on the start of the name and nothing more, so `a.md`
/// also finds `a.md.bak`'s sections; the caller keeps what belongs to it.
/// Each prefix is a range of the `C`-collated locator index, given as bounds
/// rather than a `LIKE` pattern: a parameter's pattern is unknown when the
/// statement is planned, and a range of parameters is not.
pub async fn file_topics_under(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    prefixes: &[String],
) -> Result<Vec<FileTopic>> {
    if prefixes.is_empty() {
        return Ok(Vec::new());
    }
    let (low, high): (Vec<String>, Vec<String>) = prefixes
        .iter()
        .map(|prefix| {
            let low = format!("file:{prefix}");
            // Above every string that starts with `low`: no character sorts
            // after the last one Unicode has, in UTF-8 bytes or code points.
            let high = format!("{low}{}", char::MAX);
            (low, high)
        })
        .unzip();
    let rows: Vec<(String, uuid::Uuid, Option<String>)> =
        sqlx::query_as(include_str!("../sql/repository/file_topics_under.sql"))
            .bind(project.0)
            .bind(&low)
            .bind(&high)
            .fetch_all(executor)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(name, current, root)| FileTopic {
            name,
            current: current.into(),
            root,
        })
        .collect())
}

/// Which of `topics` are sections of a file, each with the directory its file
/// was last ingested from.
pub async fn file_roots_of(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topics: &[String],
) -> Result<Vec<(String, String)>> {
    if topics.is_empty() {
        return Ok(Vec::new());
    }
    let locators: Vec<String> = topics.iter().map(|topic| format!("file:{topic}")).collect();
    let rows: Vec<(String, String)> =
        sqlx::query_as(include_str!("../sql/repository/file_roots_of.sql"))
            .bind(project.0)
            .bind(&locators)
            .fetch_all(executor)
            .await?;
    Ok(rows)
}

/// What a file looked like when an ingest read it. See `V18__file_records`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRecord {
    pub root: String,
    pub file: String,
    pub size: u64,
    pub modified_ns: i64,
    pub changed_ns: i64,
    pub inode: u64,
    pub read_ns: i64,
    pub content_hash: String,
    pub sections: u32,
    pub build: String,
}

/// The records of these files, each named from the root beside it.
pub async fn file_records_of(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    roots: &[String],
    files: &[String],
) -> Result<Vec<FileRecord>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(include_str!("../sql/repository/file_records_of.sql"))
        .bind(project.0)
        .bind(roots)
        .bind(files)
        .fetch_all(executor)
        .await?;
    Ok(rows.iter().map(file_record).collect())
}

/// Every record under `root` whose file starts with `prefix`: what a tree held
/// when it was last ingested, for a caller asking what has left it.
pub async fn file_records_under(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    root: &str,
    prefix: &str,
) -> Result<Vec<FileRecord>> {
    let rows = sqlx::query(include_str!("../sql/repository/file_records_under.sql"))
        .bind(project.0)
        .bind(root)
        .bind(prefix)
        .fetch_all(executor)
        .await?;
    Ok(rows.iter().map(file_record).collect())
}

fn file_record(row: &sqlx::postgres::PgRow) -> FileRecord {
    // Stored in a BIGINT's bits; see the migration.
    FileRecord {
        root: row.get("root"),
        file: row.get("file"),
        size: row.get::<i64, _>("size") as u64,
        modified_ns: row.get("modified_ns"),
        changed_ns: row.get("changed_ns"),
        inode: row.get::<i64, _>("inode") as u64,
        read_ns: row.get("read_ns"),
        content_hash: row.get("content_hash"),
        sections: row.get::<i32, _>("sections").max(0) as u32,
        build: row.get("build"),
    }
}

/// Records these files as read, replacing what was recorded of them.
pub async fn put_file_records(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    records: &[FileRecord],
) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    let column = |f: fn(&FileRecord) -> i64| records.iter().map(f).collect::<Vec<i64>>();
    let roots: Vec<&str> = records.iter().map(|r| r.root.as_str()).collect();
    let files: Vec<&str> = records.iter().map(|r| r.file.as_str()).collect();
    let hashes: Vec<&str> = records.iter().map(|r| r.content_hash.as_str()).collect();
    let builds: Vec<&str> = records.iter().map(|r| r.build.as_str()).collect();
    let sections: Vec<i32> = records
        .iter()
        .map(|r| i32::try_from(r.sections).unwrap_or(i32::MAX))
        .collect();
    sqlx::query(include_str!("../sql/repository/put_file_records.sql"))
        .bind(project.0)
        .bind(&roots)
        .bind(&files)
        .bind(column(|r| r.size as i64))
        .bind(column(|r| r.modified_ns))
        .bind(column(|r| r.changed_ns))
        .bind(column(|r| r.inode as i64))
        .bind(column(|r| r.read_ns))
        .bind(&hashes)
        .bind(&sections)
        .bind(&builds)
        .execute(executor)
        .await?;
    Ok(())
}

/// Forgets what was recorded of each `files[i]` under `roots[i]`.
pub async fn forget_file_records(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    roots: &[String],
    files: &[String],
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    sqlx::query(include_str!("../sql/repository/forget_file_records.sql"))
        .bind(project.0)
        .bind(roots)
        .bind(files)
        .execute(executor)
        .await?;
    Ok(())
}

/// Forgets what was recorded of these files under every root: for an ingest
/// about to rewrite their sections, which every tree naming a file alike
/// shares, so no record of them is true until the ingest writes its own.
pub async fn forget_records_of_files(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    files: &[String],
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    sqlx::query(include_str!(
        "../sql/repository/forget_records_of_files.sql"
    ))
    .bind(project.0)
    .bind(files)
    .execute(executor)
    .await?;
    Ok(())
}

/// Forgets the record of each `files[i]` under `roots[i]` one of whose
/// sections, `topics[i]`, was written by hand since `since`: a write that may
/// have landed while an ingest was recording that file, after its own
/// [`forget_file_record_of`] found no record yet to forget.
pub async fn forget_file_records_written_since(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    roots: &[String],
    files: &[String],
    topics: &[String],
    since: OffsetDateTime,
) -> Result<()> {
    if topics.is_empty() {
        return Ok(());
    }
    sqlx::query(include_str!(
        "../sql/repository/forget_file_records_written_since.sql"
    ))
    .bind(project.0)
    .bind(roots)
    .bind(files)
    .bind(topics)
    .bind(since)
    .execute(executor)
    .await?;
    Ok(())
}

/// How many sections a file's prefix range holds on the retrieval surface:
/// every file whose name starts with `prefix`, for a caller comparing it with
/// what the records under that prefix account for.
pub async fn count_file_topics_under(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    prefix: &str,
) -> Result<i64> {
    let low = format!("file:{prefix}");
    let high = format!("{low}{}", char::MAX);
    let (count,): (i64,) = sqlx::query_as(include_str!(
        "../sql/repository/count_file_topics_under.sql"
    ))
    .bind(project.0)
    .bind(&low)
    .bind(&high)
    .fetch_one(executor)
    .await?;
    Ok(count)
}

/// Forgets the record of the file `topic` was cut from, if it is a section of
/// one: a write to it other than its file's leaves the file saying something
/// its section no longer does, which only reading the file again can settle.
///
/// The file is the record whose name the topic's starts with, as a section's
/// does: the whole name, or the name and then ` › ` or ` #`.
pub async fn forget_file_record_of(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    topic: &str,
) -> Result<()> {
    sqlx::query(include_str!("../sql/repository/forget_file_record_of.sql"))
        .bind(project.0)
        .bind(topic)
        .execute(executor)
        .await?;
    Ok(())
}

/// A span's vector as it was embedded. See `V22__span_vectors`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanVector {
    /// The hash of the text the vector was embedded from.
    pub passage_hash: [u8; 32],
    /// The plan that embedded it.
    pub encoding: String,
    /// Its fp16 codes, little-endian.
    pub vector: Vec<u8>,
}

/// The vectors the model at `revision` embedded for these passages, under
/// `encoding`, or under every plan that embedded one when it is `None`: a
/// rebuild that loads no model has to choose among them.
pub async fn span_vectors(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    revision: &str,
    encoding: Option<&str>,
    passage_hashes: &[[u8; 32]],
) -> Result<Vec<SpanVector>> {
    if passage_hashes.is_empty() {
        return Ok(Vec::new());
    }
    let hashes: Vec<&[u8]> = passage_hashes.iter().map(|hash| hash.as_slice()).collect();
    let rows: Vec<(Vec<u8>, String, Vec<u8>)> =
        sqlx::query_as(include_str!("../sql/repository/span_vectors.sql"))
            .bind(project.0)
            .bind(revision)
            .bind(&hashes)
            .bind(encoding)
            .fetch_all(executor)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(hash, encoding, vector)| SpanVector {
            passage_hash: hash
                .try_into()
                .expect("span_vectors checks passage_hash is 32 bytes"),
            encoding,
            vector,
        })
        .collect())
}

/// Rows per `span_vectors` insert, so no one statement grows with the
/// project.
pub const PUT_SPAN_VECTORS_BATCH: usize = 1_000;

/// Keeps each vector, embedded by the model at `revision` under `encoding`
/// from the passage hashed beside it, [`PUT_SPAN_VECTORS_BATCH`] rows a
/// statement, all on `connection`, so a caller's transaction keeps all or
/// none. One already kept stays: the same model and plan over the same text
/// give the same vector. Unless it is of another width, which no reader
/// takes, so it is put right rather than left to cost every rebuild.
pub async fn put_span_vectors<'a>(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    revision: &str,
    encoding: &str,
    vectors: impl IntoIterator<Item = &'a ([u8; 32], Vec<u8>)>,
) -> Result<()> {
    let mut rows: Vec<(&[u8], &[u8])> = vectors
        .into_iter()
        .map(|(hash, vector)| (hash.as_slice(), vector.as_slice()))
        .collect();
    // In one order, so two keeps of overlapping spans lock their rows in the
    // same order and neither waits on a row the other holds. And once each:
    // one statement may not update a row it inserted. Which duplicate stays
    // does not matter: one hash is one text, embedded to one vector.
    rows.sort_unstable_by_key(|(hash, _)| *hash);
    rows.dedup_by_key(|(hash, _)| *hash);
    for batch in rows.chunks(PUT_SPAN_VECTORS_BATCH) {
        let (hashes, vectors): (Vec<&[u8]>, Vec<&[u8]>) = batch.iter().copied().unzip();
        sqlx::query(include_str!("../sql/repository/put_span_vectors.sql"))
            .bind(project.0)
            .bind(revision)
            .bind(encoding)
            .bind(hashes)
            .bind(vectors)
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}

/// A path `pamin-memory ingest` keeps in sync. See `V19__synced_paths`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncedPath {
    /// The project's name.
    pub project: String,
    pub path: String,
    pub vector_index: String,
}

/// Records `paths` as kept in sync for this project, under the vector index
/// the ingest asked for.
pub async fn sync_paths(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    paths: &[String],
    vector_index: &str,
) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    sqlx::query(include_str!("../sql/repository/sync_paths.sql"))
        .bind(project.0)
        .bind(paths)
        .bind(vector_index)
        .execute(executor)
        .await?;
    Ok(())
}

/// Stops keeping `paths` in sync for this project, returning those that were.
pub async fn unsync_paths(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    paths: &[String],
) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let removed: Vec<(String,)> =
        sqlx::query_as(include_str!("../sql/repository/unsync_paths.sql"))
            .bind(project.0)
            .bind(paths)
            .fetch_all(executor)
            .await?;
    Ok(removed.into_iter().map(|(path,)| path).collect())
}

/// The paths kept in sync in one project, by path.
pub async fn synced_paths_of(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
) -> Result<Vec<SyncedPath>> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as(include_str!("../sql/repository/synced_paths_of.sql"))
            .bind(project.0)
            .fetch_all(executor)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(project, path, vector_index)| SyncedPath {
            project,
            path,
            vector_index,
        })
        .collect())
}

/// Every path kept in sync, in every project, by project and path.
pub async fn synced_paths(executor: impl PgExecutor<'_>) -> Result<Vec<SyncedPath>> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as(include_str!("../sql/repository/synced_paths.sql"))
            .fetch_all(executor)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(project, path, vector_index)| SyncedPath {
            project,
            path,
            vector_index,
        })
        .collect())
}

/// A topic as it stands now, found by name.
#[derive(Clone, Debug)]
pub struct Named {
    pub name: String,
    pub topic: TopicId,
    pub current: TopicStateId,
    pub content: String,
}

/// Each of the topics `names` as it stands now; a name with no topic, or none
/// on the retrieval surface, is left out.
///
/// For an ingest, which asks it about many sections at once: to number a
/// file's sections against what they said before, and to resolve the
/// sections its outline relates.
pub async fn current_named(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    names: &[String],
) -> Result<Vec<Named>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(include_str!("../sql/repository/current_named.sql"))
        .bind(project.0)
        .bind(names)
        .fetch_all(executor)
        .await?;
    Ok(rows
        .iter()
        .map(|row| Named {
            name: row.get("name"),
            topic: row.get::<uuid::Uuid, _>("id").into(),
            current: row.get::<uuid::Uuid, _>("state_id").into(),
            content: content_of_span(row),
        })
        .collect())
}

/// Renames a topic, keeping the name it had as an alias that
/// [`find_topic`] resolves, and files the source at `from_locator` under
/// `to_locator`; then owes the projection the topic again, since its name is
/// read into every span's header.
///
/// Returns false, changing nothing, when another topic or source already holds
/// the new name. For a topic locked by the caller's transaction.
pub async fn rename_topic(
    connection: &mut sqlx::PgConnection,
    project: ProjectId,
    topic: TopicId,
    (from, to): (&str, &str),
    (from_locator, to_locator): (&str, &str),
) -> Result<bool> {
    let taken: Option<(i32,)> = sqlx::query_as(include_str!(
        "../sql/repository/topic_name_or_locator_taken.sql"
    ))
    .bind(project.0)
    .bind(to)
    .bind(to_locator)
    .fetch_optional(&mut *connection)
    .await?;
    if taken.is_some() {
        return Ok(false);
    }
    sqlx::query(include_str!("../sql/repository/rename_topic.sql"))
        .bind(topic.0)
        .bind(to)
        .execute(&mut *connection)
        .await?;
    sqlx::query(include_str!("../sql/repository/rename_source_locator.sql"))
        .bind(project.0)
        .bind(from_locator)
        .bind(to_locator)
        .execute(&mut *connection)
        .await?;
    // The new name is a topic's own now, and the old one is this topic's.
    sqlx::query(include_str!("../sql/repository/move_topic_alias.sql"))
        .bind(project.0)
        .bind(from)
        .bind(to)
        .bind(topic.0)
        .bind(OffsetDateTime::now_utc())
        .execute(&mut *connection)
        .await?;
    crate::jobs::enqueue_all(
        &mut *connection,
        project,
        &[JobKind::SyncTopicIndex],
        Some(topic.0),
    )
    .await?;
    Ok(true)
}

/// [`set_source_root`] for every source filed under `locators`, in one
/// statement: for the sections an ingest found unchanged, which write nothing
/// else, so that each records the tree it was last ingested from all the same.
pub async fn set_roots_at(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    locators: &[String],
    root: &str,
) -> Result<()> {
    if locators.is_empty() {
        return Ok(());
    }
    sqlx::query(include_str!("../sql/repository/set_roots_at.sql"))
        .bind(project.0)
        .bind(locators)
        .bind(root)
        .execute(executor)
        .await?;
    Ok(())
}

/// Records `root` as the directory the file source was last ingested from.
pub async fn set_source_root(
    executor: impl PgExecutor<'_>,
    source: SourceId,
    root: &str,
) -> Result<()> {
    sqlx::query(include_str!("../sql/repository/set_source_root.sql"))
        .bind(source.0)
        .bind(root)
        .execute(executor)
        .await?;
    Ok(())
}

/// Lists every topic in a project.
///
/// Used to derive relationships from one topic's content naming another, which
/// needs the whole set rather than a candidate list: any topic may be named.
pub async fn all_topics(executor: impl PgExecutor<'_>, project: ProjectId) -> Result<Vec<Topic>> {
    let rows = sqlx::query(include_str!("../sql/repository/all_topics.sql"))
        .bind(project.0)
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| Topic {
            id: row.get::<uuid::Uuid, _>("id").into(),
            project_id: project,
            name: row.get("name"),
            path: row.get("path"),
            created_at: row.get("created_at"),
        })
        .collect())
}

/// The topics written most recently, newest first.
///
/// Bounded, and ordered by an index rather than by sorting the project: the
/// caller wants to know what is here, and a caller that wanted all of it would
/// be asking for something no context window can hold anyway.
pub async fn recent_topics(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    limit: u32,
) -> Result<Vec<Topic>> {
    let rows = sqlx::query(include_str!("../sql/repository/recent_topics.sql"))
        .bind(project.0)
        .bind(i64::from(limit))
        .fetch_all(executor)
        .await?;

    Ok(rows
        .iter()
        .map(|row| Topic {
            id: row.get::<uuid::Uuid, _>("id").into(),
            project_id: project,
            name: row.get("name"),
            path: row.get("path"),
            created_at: row.get("created_at"),
        })
        .collect())
}

/// How many topics a project holds.
///
/// The projection is keyed by topic, so this is how many documents it will
/// hold once it has caught up -- which is what the index needs to size its
/// segments before it is opened.
pub async fn topic_count(executor: impl PgExecutor<'_>, project: ProjectId) -> Result<u64> {
    let count: i64 = sqlx::query_scalar(include_str!("../sql/repository/topic_count.sql"))
        .bind(project.0)
        .fetch_one(executor)
        .await?;
    Ok(count.max(0) as u64)
}

// A topic by its name, or by a name it went by before a rename when no topic
// holds that name now: the topic's own row, read under its current name.
const FIND_TOPIC: &str = include_str!("../sql/repository/find_topic.sql");

/// Looks up a topic by name within a project, or by a name it was renamed
/// from that no topic holds now; the topic comes back under its current name.
pub async fn find_topic(
    executor: impl PgExecutor<'_>,
    project: ProjectId,
    name: &str,
) -> Result<Option<Topic>> {
    let row = sqlx::query(FIND_TOPIC)
        .bind(project.0)
        .bind(name)
        .fetch_optional(executor)
        .await?;

    Ok(row.map(|row| Topic {
        id: row.get::<uuid::Uuid, _>("id").into(),
        project_id: project,
        name: row.get("name"),
        path: row.get("path"),
        created_at: row.get("created_at"),
    }))
}
