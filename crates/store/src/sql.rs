//! Mapping domain enums onto the TEXT columns that store them.
//!
//! Every such column carries a CHECK constraint naming the same set of labels,
//! so the SQL and the Rust have to agree. Writing the two directions by hand
//! per enum is how they stop agreeing: a variant gets added, one of the two
//! matches gets updated, and the mismatch surfaces as a value that reads back
//! as something else. Here one list generates both directions, and a test
//! holds the SQL to the same list.

/// A domain enum with a stable textual representation in the database.
pub(crate) trait SqlLabel: Sized {
    /// The label written to the column.
    fn label(self) -> &'static str;

    /// Reads a label back.
    ///
    /// `None` means the column held something its CHECK constraint should have
    /// rejected. Callers decide what to do about that; nothing here guesses.
    fn from_label(label: &str) -> Option<Self>;

    /// Every label this enum can produce. Exists for the drift test below.
    #[cfg(test)]
    fn labels() -> &'static [&'static str];
}

macro_rules! sql_enum {
    ($ty:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        impl crate::sql::SqlLabel for $ty {
            fn label(self) -> &'static str {
                match self {
                    $($ty::$variant => $label,)+
                }
            }

            fn from_label(label: &str) -> Option<Self> {
                match label {
                    $($label => Some($ty::$variant),)+
                    _ => None,
                }
            }

            #[cfg(test)]
            fn labels() -> &'static [&'static str] {
                &[$($label),+]
            }
        }
    };
}

pub(crate) use sql_enum;

#[cfg(test)]
mod tests {
    use super::SqlLabel;
    use memory_core::{Derivation, EdgeKind, FilterDecision, JobKind, SourceKind, TombstoneReason};

    /// Enums whose column is constrained to their label set.
    fn constrained() -> Vec<&'static [&'static str]> {
        vec![
            FilterDecision::labels(),
            SourceKind::labels(),
            EdgeKind::labels(),
            Derivation::labels(),
            TombstoneReason::labels(),
            JobKind::labels(),
        ]
    }

    fn migrations() -> String {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        std::fs::read_dir(dir)
            .expect("migrations directory")
            .filter_map(|entry| {
                let path = entry.expect("directory entry").path();
                (path.extension()? == "sql").then(|| std::fs::read_to_string(&path).expect("sql"))
            })
            .collect()
    }

    /// A label the schema rejects is a value that cannot be written, and a
    /// constraint listing a label no variant produces is a rule nothing
    /// enforces. Both are silent: each half compiles and runs on its own.
    #[test]
    fn every_constrained_label_appears_in_the_schema() {
        let sql = migrations();
        let mut checked = 0;

        for labels in constrained() {
            for label in labels {
                assert!(
                    sql.contains(&format!("'{label}'")),
                    "no migration admits the label {label:?}"
                );
                checked += 1;
            }
        }

        assert!(checked > 0, "no labels were checked");
    }

    fn select_list(sql: &str) -> String {
        let sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");
        let (columns, _) = sql
            .strip_prefix("SELECT ")
            .and_then(|query| query.split_once(" FROM "))
            .expect("expected a SELECT projection");
        match columns.strip_prefix("DISTINCT ON (") {
            Some(distinct) => distinct
                .split_once(") ")
                .expect("expected columns after DISTINCT ON")
                .1
                .to_owned(),
            None => columns.to_owned(),
        }
    }

    #[test]
    fn relationship_version_queries_project_the_same_version_columns() {
        let expected = select_list(include_str!("../sql/graph/live_version.sql"));
        for (name, sql) in [
            (
                "edge_history",
                include_str!("../sql/graph/edge_history.sql"),
            ),
            (
                "live_versions_for_relationships",
                include_str!("../sql/graph/live_versions_for_relationships.sql"),
            ),
        ] {
            assert_eq!(select_list(sql), expected, "{name}");
        }

        let qualified = expected
            .split(", ")
            .map(|column| format!("v.{column}"))
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(
            select_list(include_str!("../sql/graph/live_versions_of.sql")),
            format!("r.from_topic, r.to_topic, r.kind, {qualified}"),
            "live_versions_of",
        );

        let append = include_str!("../sql/graph/append_relationship_versions.sql");
        let (_, returning) = append
            .rsplit_once("RETURNING ")
            .expect("append must return the inserted versions");
        assert_eq!(
            returning.split_whitespace().collect::<Vec<_>>().join(" "),
            expected,
            "append_relationship_versions",
        );
    }

    #[test]
    fn topic_state_queries_project_the_same_state_and_span_columns() {
        let expected = select_list(include_str!("../sql/repository/topic_state.sql"));
        for (name, sql) in [
            (
                "all_current_topic_states",
                include_str!("../sql/repository/all_current_topic_states.sql"),
            ),
            (
                "newest_surviving_state",
                include_str!("../sql/repository/newest_surviving_state.sql"),
            ),
        ] {
            assert_eq!(select_list(sql), expected, "{name}");
        }
        assert_eq!(
            select_list(include_str!("../sql/repository/current_states_named.sql")),
            format!("{expected}, t.name AS topic_name"),
            "current_states_named",
        );
    }
}
