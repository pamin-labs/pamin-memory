//! The engine's settings, from the server settings a command resolved.
//!
//! Resolving them -- flags over `config.toml` over the defaults -- is the
//! client's, in `memory_client::settings`, because every command does it and
//! the client links none of the libraries. What is left here is handing the
//! result to the engine, with the libraries' own defaults for everything a
//! server is not started with.

use memory_client::settings::{DATABASE_URL, Server};

/// What the engine runs with, for a server resolved as `server` and the
/// external database `url`, if one is named. The URL is read from the
/// environment alone, by [`database_url`]: it holds credentials, which belong
/// in neither a flag, `config.toml`, nor the settings a client sends the
/// server and prints.
pub fn engine(
    server: &Server,
    url: Option<memory_store::DatabaseUrl>,
) -> anyhow::Result<memory_engine::Settings> {
    let default = memory_engine::Settings::default();
    Ok(memory_engine::Settings {
        inference: memory_index::InferenceSettings {
            device: server.device,
            threads: server.threads,
            ep_libraries: server.ep_libraries.clone(),
            prepared: server.prepared,
            fused_attention: server.fused_attention,
            split_embeddings: server.split_embeddings,
            ..default.inference
        },
        store: store(server, url)?,
        model_idle: server.model_idle,
        models_dir: server.models_dir.clone(),
        ..default
    })
}

/// The store settings for `server` and the external database `url`, if one is
/// named. A cap on connections to an external database is refused without
/// one, rather than ignored.
fn store(
    server: &Server,
    url: Option<memory_store::DatabaseUrl>,
) -> anyhow::Result<memory_store::StoreSettings> {
    if url.is_none() && server.database_connections.is_some() {
        anyhow::bail!(
            "database_connections caps the connections to the server named by {DATABASE_URL}, \
             which is not set; set it, or drop the cap"
        );
    }
    Ok(memory_store::StoreSettings {
        jit: server.jit,
        postgres_dir: server.postgres_dir.clone(),
        external: url.map(|url| memory_store::External {
            url,
            connections: server.database_connections,
        }),
    })
}

/// The external database the server opens, if one is named; refused when it is
/// named but unusable, since the fallback, the workspace's own cluster, would
/// keep a hosted node's data on a disk that is not where its data lives.
pub(crate) fn database_url() -> anyhow::Result<Option<memory_store::DatabaseUrl>> {
    match std::env::var(DATABASE_URL) {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => anyhow::bail!("{DATABASE_URL} is not UTF-8"),
        Ok(url) if url.trim().is_empty() => {
            anyhow::bail!("{DATABASE_URL} is empty; unset it to use the workspace's own cluster")
        }
        Ok(url) => Ok(Some(memory_store::DatabaseUrl::new(url))),
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, FromArgMatches, Parser};
    use memory_client::settings::{OPEN_INDEXES, ServerArgs};

    #[derive(Parser)]
    struct Test {
        #[command(flatten)]
        server: ServerArgs,
    }

    /// A command that sets nothing runs the engine at the libraries' own
    /// defaults: the client resolves against the defaults it shares with
    /// them, so the two cannot drift apart unnoticed.
    #[test]
    fn nothing_set_is_the_library_defaults() {
        let root = tempfile::tempdir().unwrap();
        let matches = Test::command()
            .try_get_matches_from(["pamin-memory"])
            .unwrap();
        let parsed = Test::from_arg_matches(&matches).unwrap();
        let server =
            memory_client::settings::Server::resolve(root.path(), &parsed.server, &matches)
                .unwrap();
        // `PAMIN_MEMORY_DEVICE` may be set by whoever runs the tests; nothing else is
        // read from the environment.
        let settings = memory_engine::Settings {
            inference: memory_index::InferenceSettings {
                device: server.device,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(super::engine(&server, None).unwrap(), settings);
        assert_eq!(server.open_indexes, OPEN_INDEXES);
    }

    /// A connection cap names no server without an external database, so it
    /// is refused rather than ignored, and taken with one.
    #[test]
    fn a_connection_cap_needs_an_external_database() {
        let root = tempfile::tempdir().unwrap();
        let matches = Test::command()
            .try_get_matches_from(["pamin-memory", "--database-connections", "2"])
            .unwrap();
        let parsed = Test::from_arg_matches(&matches).unwrap();
        let server =
            memory_client::settings::Server::resolve(root.path(), &parsed.server, &matches)
                .unwrap();
        let error = super::store(&server, None).unwrap_err().to_string();
        assert!(
            error.contains(memory_client::settings::DATABASE_URL),
            "{error}"
        );
        let url = memory_store::DatabaseUrl::new("postgres://node@db/memory".to_string());
        let store = super::store(&server, Some(url)).unwrap();
        assert_eq!(
            store.external.unwrap().connections,
            std::num::NonZeroU32::new(2)
        );
    }
}
