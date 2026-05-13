//! Clap definitions for the `stoa` CLI.
//!
//! Subcommands map 1-to-1 onto sibling modules (`init`, `read`, `write`,
//! `schema`). Each handler is responsible for its own output + exit code.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Open-core knowledge + memory system for AI agents.
#[derive(Parser, Debug)]
#[command(
    name = "stoa",
    version,
    about = ABOUT,
    long_about = None,
    disable_help_subcommand = true,
    disable_help_flag = true,
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// Crate-level about line surfaced in `--help`.
pub(crate) const ABOUT: &str = "Open-core knowledge + memory system for AI agents.";

/// Custom help body emitted by `stoa --help`. Pinned by the
/// `tests/cmd/help.trycmd` golden snapshot.
pub(crate) const HELP_BODY: &str = "\
Open-core knowledge + memory system for AI agents.

  init    Scaffold a fresh Stoa workspace (idempotent)
  read    Print a wiki page
  write   Create or update a wiki page
  schema  Print or validate the workspace schema
  query   Hybrid recall over the indexed corpus
  index   Manage the recall index (FTS5 + KG)
  inject  Inspect SessionStart injection history

";

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Scaffold a fresh Stoa workspace (idempotent).
    Init {
        /// Skip Python sidecar bootstrap; BM25-only workspace (<5s cold start).
        #[arg(long)]
        no_embeddings: bool,
    },

    /// Print a wiki page (frontmatter + body) to stdout.
    Read {
        /// Page id, e.g. `ent-redis` / `con-rag` / `syn-…`.
        id: String,
    },

    /// Create or update a wiki page.
    Write {
        /// Page id; routed by prefix (`ent-`/`con-`/`syn-`).
        id: String,
        /// Path to a YAML frontmatter file (optional on update).
        #[arg(long)]
        frontmatter: Option<PathBuf>,
        /// Path to a markdown body file (optional on update).
        #[arg(long)]
        body: Option<PathBuf>,
    },

    /// Print or validate the workspace schema (`STOA.md`).
    Schema {
        /// Validate every wiki page against the schema instead of printing.
        #[arg(long)]
        check: bool,
    },

    /// Run the capture daemon (or a single drain cycle with `--once`).
    Daemon {
        /// Drain one row and exit instead of running the long-lived loop.
        #[arg(long)]
        once: bool,
    },

    /// Manage Stoa hooks for an agent platform.
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Hybrid recall over the indexed corpus.
    Query {
        /// Query string.
        q: String,
        /// Emit JSON `{hits: [...]}` instead of human-readable lines.
        #[arg(long)]
        json: bool,
        /// Comma-separated streams: `vector`, `bm25`, `graph`. Default: all.
        #[arg(long, value_delimiter = ',')]
        streams: Vec<String>,
        /// Cap on returned hits.
        #[arg(long, default_value_t = 10)]
        k: usize,
    },

    /// Manage the recall index (FTS5 + KG).
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },

    /// Inspect `SessionStart` injection history (`.stoa/audit.log`).
    Inject {
        #[command(subcommand)]
        action: InjectAction,
    },
}

/// `stoa hook ...` sub-subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum HookAction {
    /// Print the registration snippet for the given platform.
    Install {
        /// Target platform (e.g. `claude-code`).
        #[arg(long)]
        platform: String,
        /// Emit the `SessionStart` injection snippet instead of the
        /// capture snippet (e.g. `--inject session-start`).
        #[arg(long, value_name = "KIND")]
        inject: Option<String>,
    },
}

/// `stoa inject ...` sub-subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum InjectAction {
    /// Print injection history from `.stoa/audit.log`.
    Log {
        /// Restrict to events for a single session id.
        #[arg(long)]
        session: Option<String>,
        /// Cap on returned events (most recent first).
        #[arg(long)]
        limit: Option<usize>,
    },
}

/// `stoa index ...` sub-subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum IndexAction {
    /// Drop and rebuild the FTS5 + vector index from `wiki/` + `sessions/`.
    Rebuild,
}

impl Cli {
    /// Dispatch to the relevant subcommand handler.
    pub(crate) fn dispatch(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init { no_embeddings } => crate::init::run(no_embeddings),
            Command::Read { id } => crate::read::run(&id),
            Command::Write { id, frontmatter, body } => {
                crate::write::run(&id, frontmatter.as_deref(), body.as_deref())
            },
            Command::Schema { check } => crate::schema::run(check),
            Command::Daemon { once } => crate::daemon::run(once),
            Command::Hook { action } => match action {
                HookAction::Install { platform, inject } => {
                    crate::hook::install(&platform, inject.as_deref())
                },
            },
            Command::Query { q, json, streams, k } => crate::query::run(&q, json, &streams, k),
            Command::Index { action } => match action {
                IndexAction::Rebuild => crate::index::rebuild(),
            },
            Command::Inject { action } => match action {
                InjectAction::Log { session, limit } => {
                    crate::inject::log(session.as_deref(), limit)
                },
            },
        }
    }
}
