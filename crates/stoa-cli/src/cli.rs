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
/// `tests/cmd/help.trycmd` golden snapshot — 7 lines + trailing newline.
pub(crate) const HELP_BODY: &str = "\
Open-core knowledge + memory system for AI agents.

  init    Scaffold a fresh Stoa workspace (idempotent)
  read    Print a wiki page
  write   Create or update a wiki page
  schema  Print or validate the workspace schema

";

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Scaffold a fresh Stoa workspace (idempotent).
    Init,

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
}

/// `stoa hook ...` sub-subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum HookAction {
    /// Print the registration snippet for the given platform.
    Install {
        /// Target platform (e.g. `claude-code`).
        #[arg(long)]
        platform: String,
    },
}

impl Cli {
    /// Dispatch to the relevant subcommand handler.
    pub(crate) fn dispatch(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init => crate::init::run(),
            Command::Read { id } => crate::read::run(&id),
            Command::Write { id, frontmatter, body } => {
                crate::write::run(&id, frontmatter.as_deref(), body.as_deref())
            },
            Command::Schema { check } => crate::schema::run(check),
            Command::Daemon { once } => crate::daemon::run(once),
            Command::Hook { action } => match action {
                HookAction::Install { platform } => crate::hook::install(&platform),
            },
        }
    }
}
