//! A store on a PostgreSQL server it did not start, as hosted memory runs:
//! a deploy migrates it with `Database::migrate`, `StoreSettings::external`
//! connects to it, and the workspace's own cluster is neither started nor
//! recorded.
//!
//! Run with `cargo test -p memory-store --test external -- --ignored`.

mod common;

use std::num::NonZeroU32;

use memory_store::{Connections, Database, DatabaseUrl, External, StoreSettings, Workspace};

#[tokio::test]
#[ignore = "needs a real postgres cluster"]
async fn an_external_server_is_used_and_never_started() {
    // A cluster to stand in for the hosted one: another workspace's own.
    let host = tempfile::tempdir().expect("host workspace");
    let host = Workspace::at(host.path());
    let hosted = Database::open(&host, Connections::PerCommand, &common::store())
        .await
        .expect("start the stand-in server");
    let record = host
        .read_server()
        .expect("read the host's record")
        .expect("the host recorded its server");
    let superuser = record.url();
    let (server, _) = superuser.rsplit_once('/').expect("a database in the URL");
    let (_, address) = server.split_once('@').expect("credentials in the URL");
    let on = |url: String| StoreSettings {
        external: Some(External {
            url: DatabaseUrl::new(url),
            connections: None,
        }),
        ..common::store()
    };
    let external =
        |role: &str, database: &str| on(format!("postgresql://{role}:{role}@{address}/{database}"));

    // A hosted login: it owns its databases and is bound by row-level
    // security, with the guarded role made for it once. And one with
    // BYPASSRLS, as a managed server's administrator often is, given node's
    // privileges so it can open and migrate the database node owns.
    for statement in [
        "CREATE ROLE node LOGIN PASSWORD 'node' NOSUPERUSER NOBYPASSRLS",
        "GRANT pamin_memory_app TO node WITH ADMIN OPTION",
        "CREATE DATABASE nodes OWNER node",
        "CREATE ROLE bypassing LOGIN PASSWORD 'bypassing' NOSUPERUSER BYPASSRLS",
        "GRANT node TO bypassing",
    ] {
        run(&hosted, statement).await;
    }

    let node = tempfile::tempdir().expect("node workspace");
    let node = Workspace::at(node.path());
    let settings = external("node", "nodes");

    // A server never migrates the database it shares: until a deploy has, it
    // refuses to start, naming what is missing.
    let behind = Database::open(&node, Connections::Resident, &settings)
        .await
        .err()
        .expect("a database nothing has migrated is refused");
    assert!(!missing(&behind).is_empty(), "{behind}");
    assert!(
        behind.to_string().contains("pamin-memory migrate"),
        "{behind}"
    );
    let url = &settings.external.as_ref().expect("an external server").url;
    let applied = Database::migrate(url)
        .await
        .expect("migrate as a deploy does");
    assert_eq!(applied, *missing(&behind), "it applies what was missing");
    let again = Database::migrate(url)
        .await
        .expect("a second deploy migrates nothing new and fails nothing");
    assert!(again.is_empty(), "{again:?}");

    let database = Database::open(&node, Connections::Resident, &settings)
        .await
        .expect("open the external server");
    assert!(
        node.read_server()
            .expect("read the node's record")
            .is_none(),
        "no cluster is started or recorded for the node"
    );

    // One database: what the node writes, the host reads.
    let name = format!("external-{}", uuid::Uuid::now_v7());
    let written = memory_store::repository::ensure_project(database.pool(), &name)
        .await
        .expect("write a project through the node");
    let as_host = sqlx::PgPool::connect(&format!("{server}/nodes"))
        .await
        .expect("the host on the node's database");
    let read = memory_store::repository::ensure_project(&as_host, &name)
        .await
        .expect("read it through the host");
    assert_eq!(written.id, read.id);

    // The URL carries credentials and never prints.
    let printed = format!("{settings:?}");
    assert!(!printed.contains("node:node"), "{printed}");

    // A node's pool is capped, so nodes sharing the server fit its limit.
    let capped = StoreSettings {
        external: Some(External {
            url: url.clone(),
            connections: NonZeroU32::new(2),
        }),
        ..common::store()
    };
    let database = Database::open(&node, Connections::Resident, &capped)
        .await
        .expect("open with a cap");
    assert_eq!(database.pool().options().get_max_connections(), 2);

    // A hosted login and the guarded role skip nothing.
    let skipping = |database: Database| async move {
        database
            .skipping_row_level_security()
            .await
            .expect("read the roles' attributes")
    };
    assert!(skipping(database).await.is_empty());

    // A login row-level security would not bind is the operator's choice: a
    // server and a deploy migrating both run, and it is named, for the warning.
    // The one with BYPASSRLS, and a superuser.
    for (login, role) in [
        (external("bypassing", "nodes"), "bypassing"),
        (on(format!("{server}/nodes")), record.username.as_str()),
    ] {
        let database = Database::open(&node, Connections::PerCommand, &login)
            .await
            .unwrap_or_else(|error| panic!("open as {role}: {error}"));
        assert_eq!(skipping(database).await, [role]);
        let url = &login.external.as_ref().expect("an external server").url;
        Database::migrate(url)
            .await
            .unwrap_or_else(|error| panic!("migrate as {role}: {error}"));
    }

    // So is a guarded role an operator made with BYPASSRLS.
    run(&hosted, "ALTER ROLE pamin_memory_app BYPASSRLS").await;
    let database = Database::open(&node, Connections::PerCommand, &settings)
        .await
        .expect("open with a guarded role that bypasses");
    assert_eq!(skipping(database).await, ["pamin_memory_app"]);
}

/// Runs a statement on the stand-in server as its superuser.
async fn run(hosted: &Database, statement: &'static str) {
    sqlx::query(statement)
        .execute(hosted.pool())
        .await
        .unwrap_or_else(|error| panic!("{statement}: {error}"));
}

/// The versions a refused open said were missing.
fn missing(error: &memory_store::StoreError) -> &Vec<i64> {
    match error {
        memory_store::StoreError::Behind(pending) => pending,
        other => panic!("refused for another reason: {other}"),
    }
}
