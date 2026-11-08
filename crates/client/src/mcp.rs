//! `pamin-memory mcp`: the operation contract served as MCP over standard input and
//! output, for agents that load tools rather than run commands.
//!
//! Each tool is one request to the workspace's server, made the way the
//! command line makes it: the same contract types, the same socket, the same
//! server started on first use. What a call works on comes from the kit's
//! identity seam, which locally is the project `--project` names, and never
//! from a tool's arguments.

use std::collections::BTreeMap;
use std::sync::Arc;

use memory_contract::{grep, link, neighbors, read, search, topics, unlink, write};
use memory_core::Workspace;
use pamin_kit::{
    Audience, Ctx, Feature, Hardware, Hints, Kit, Local, Manifest, Module, Ops, Routing,
};
use serde::de::DeserializeOwned;

use crate::protocol::{self, Call, Request};
use crate::{client, settings};

/// The names of the tools this module serves: what an MCP client calls, and
/// what whoever hosts memory meters each call by.
pub mod tool {
    pub const SEARCH: &str = "memory_search";
    pub const READ: &str = "memory_read";
    pub const GREP: &str = "memory_grep";
    pub const NEIGHBORS: &str = "memory_neighbors";
    pub const TOPICS: &str = "memory_topics";
    pub const WRITE: &str = "memory_write";
    pub const LINK: &str = "memory_link";
    pub const UNLINK: &str = "memory_unlink";
}

/// The tools that only read the project.
const READS: [&str; 5] = [
    tool::SEARCH,
    tool::READ,
    tool::GREP,
    tool::NEIGHBORS,
    tool::TOPICS,
];
const READ_ONLY: Hints = Hints {
    read_only: true,
    destructive: false,
    idempotent: true,
    open_world: false,
};
const ADD: Hints = Hints {
    read_only: false,
    destructive: false,
    idempotent: false,
    open_world: false,
};

/// Memory as a pamin-kit module: what a request needs besides its call.
pub struct Memory {
    workspace: Workspace,
    vector_index: String,
    /// The server settings every request asks for, as the CLI resolved them.
    server: BTreeMap<String, settings::Asked>,
    /// The flags a server started for a tool call is started with.
    serve: Vec<std::ffi::OsString>,
}

impl Memory {
    pub fn new(
        workspace: Workspace,
        vector_index: String,
        server: BTreeMap<String, settings::Asked>,
        serve: Vec<std::ffi::OsString>,
    ) -> Self {
        Self {
            workspace,
            vector_index,
            server,
            serve,
        }
    }

    /// Sends `call` for the caller's project and reads the answer as `R`.
    ///
    /// The error is a message, all of its causes included, because a model
    /// reads it as the tool's result and has nothing else to go on.
    async fn ask<R: DeserializeOwned>(self: Arc<Self>, ctx: Ctx, call: Call) -> Result<R, String> {
        let request = Request {
            version: protocol::version(),
            wire: protocol::WIRE.to_string(),
            project: ctx.space().project.clone(),
            profile: String::new(),
            vector_index: self.vector_index.clone(),
            server: self.server.clone(),
            call,
        };
        let answer = client::ask(&self.workspace, &request, &self.serve)
            .await
            .map_err(|error| format!("{error:#}"))?
            // `None` only for `stop`, which is not a tool.
            .ok_or("no server answered")?;
        serde_json::from_str(answer.get()).map_err(|error| format!("reading the answer: {error}"))
    }
}

impl Module for Memory {
    fn manifest(&self) -> Manifest {
        Manifest {
            name: "memory",
            contract_version: memory_contract::VERSION,
            routing: Routing::ShardedByProject,
            long_running: false,
            scopes: vec![],
            metering_unit: "call",
            hardware: Hardware::default(),
            audience: Audience::Model,
        }
    }

    fn register(&self, ops: &mut Ops<Self>) {
        ops.tool(tool::SEARCH, Feature::Core, |m, ctx, a| {
            m.ask::<search::Results>(ctx, Call::Search(a))
        })
        .tool(tool::READ, Feature::Core, |m, ctx, a| {
            m.ask::<read::Read>(ctx, Call::Read(a))
        })
        .tool(tool::GREP, Feature::Core, |m, ctx, a| {
            m.ask::<grep::Matches>(ctx, Call::Grep(a))
        })
        .tool(tool::NEIGHBORS, Feature::Core, |m, ctx, a| {
            m.ask::<neighbors::Neighborhood>(ctx, Call::Neighbors(a))
        })
        .tool(tool::TOPICS, Feature::Core, |m, ctx, a| {
            m.ask::<topics::Topics>(ctx, Call::Topics(a))
        })
        .tool(tool::WRITE, Feature::Core, |m, ctx, a| {
            m.ask::<write::Written>(ctx, Call::Write(a))
        })
        .tool(tool::LINK, Feature::Core, |m, ctx, a| {
            m.ask::<link::Linked>(ctx, Call::Link(a))
        })
        .tool(tool::UNLINK, Feature::Core, |m, ctx, a| {
            m.ask::<unlink::Unlinked>(ctx, Call::Unlink(a))
        });
        for name in READS {
            ops.hints(name, READ_ONLY);
        }
        // Writing and linking add a version and keep the old one.
        ops.hints(tool::WRITE, ADD).hints(tool::LINK, ADD);
        // Unlinking with `deleted` erases that a link ever held.
        ops.hints(
            tool::UNLINK,
            Hints {
                destructive: true,
                ..ADD
            },
        );
    }
}

/// Serves `memory` for `project` until the client disconnects.
pub async fn serve(memory: Memory, project: String) -> std::io::Result<()> {
    Kit::new(memory, Local::new(project)).serve_stdio().await
}
