//! The store's statements live in `.sql` files, and data.md forbids fragments,
//! so a column list that several queries project is written in each. These
//! tests hold every copy to the first.

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
