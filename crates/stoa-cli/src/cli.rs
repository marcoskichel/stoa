//! Clap definitions for the `stoa` CLI surface.
//!
//! Verbs map 1:1 to functions in [`crate::commands`]. The argument
//! shapes are stable enough that downstream tooling can pin against
//! them; semver discipline applies to additions, not removals.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(
    name = "stoa",
    version,
    about = "Stoa: MemPalace-backed memory + LLM wiki for AI agents."
)]
pub(crate) struct Cli {
    /// Subcommand to dispatch.
    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {
    /// Wrapper around [`clap::Parser::parse`] for the binary entry point.
    pub(crate) fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Scaffold a fresh workspace (STOA.md + wiki/* + .stoa/).
    Init(InitArgs),

    /// Long-lived daemon lifecycle.
    Daemon {
        /// Daemon action to take.
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Print Claude Code hook configuration snippets.
    Hook {
        /// Hook action.
        #[command(subcommand)]
        action: HookAction,
    },

    /// Print or validate the active `STOA.md` schema.
    Schema(SchemaArgs),

    /// Write (or overwrite) a wiki page.
    Write(WriteArgs),

    /// Read a wiki page (frontmatter + body) back from disk.
    Read(ReadArgs),

    /// Query the wiki + verbatim drawers via the daemon.
    Query(QueryArgs),

    /// Inspect or tail the injection audit log.
    Inject {
        /// Inject action.
        #[command(subcommand)]
        action: InjectAction,
    },
}

/// `stoa init` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct InitArgs {
    /// Directory to scaffold (defaults to current dir).
    #[arg(default_value = ".")]
    pub(crate) dir: PathBuf,
}

/// `stoa daemon` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum DaemonAction {
    /// Spawn the daemon in the background.
    Start,
    /// Stop the daemon (SIGTERM).
    Stop,
    /// Health-probe the daemon over its socket.
    Status,
}

/// `stoa hook` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum HookAction {
    /// Print the Claude Code hook installation snippet.
    Install(HookInstallArgs),
}

/// `stoa hook install` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct HookInstallArgs {
    /// Agent platform to emit a snippet for.
    #[arg(long, default_value = "claude-code")]
    pub(crate) platform: String,
    /// Include `UserPromptSubmit` + `SessionStart` inject hooks.
    #[arg(long, default_value_t = true)]
    pub(crate) inject: bool,
}

/// `stoa schema` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct SchemaArgs {
    /// Validate the wiki against STOA.md instead of printing it.
    #[arg(long)]
    pub(crate) check: bool,
}

/// `stoa write` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct WriteArgs {
    /// Page id (e.g. `ent-redis`).
    pub(crate) page_id: String,
    /// Path to a YAML file holding the page frontmatter.
    #[arg(long)]
    pub(crate) frontmatter: PathBuf,
    /// Path to a markdown file holding the page body.
    #[arg(long)]
    pub(crate) body: PathBuf,
}

/// `stoa read` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct ReadArgs {
    /// Page id.
    pub(crate) page_id: String,
}

/// `stoa query` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct QueryArgs {
    /// Free-text query.
    pub(crate) query: String,
    /// Max hits to return.
    #[arg(long, default_value_t = 5)]
    pub(crate) top_k: usize,
    /// Drop the default `kind=wiki` filter (returns drawer hits too).
    #[arg(long)]
    pub(crate) include_drawers: bool,
}

/// `stoa inject` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum InjectAction {
    /// Tail the injection audit log.
    Log(InjectLogArgs),
}

/// `stoa inject log` arguments.
#[derive(Debug, clap::Args)]
pub(crate) struct InjectLogArgs {
    /// Max rows to print.
    #[arg(long, default_value_t = 20)]
    pub(crate) limit: usize,
    /// Filter rows to a specific session id.
    #[arg(long)]
    pub(crate) session: Option<String>,
}
