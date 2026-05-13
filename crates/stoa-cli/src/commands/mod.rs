//! CLI command dispatch.

mod daemon;
mod hook;
mod init;
mod inject;
mod query;
mod read;
mod runtime;
mod schema;
mod write;

use anyhow::Result;

use crate::cli::{Cli, Command, DaemonAction, HookAction, InjectAction};

/// Route the parsed CLI to its command implementation.
pub(crate) fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init(args) => init::run(args),
        Command::Daemon { action } => match action {
            DaemonAction::Start => daemon::start(),
            DaemonAction::Stop => daemon::stop(),
            DaemonAction::Status => runtime::block_on(daemon::status()),
        },
        Command::Hook { action } => match action {
            HookAction::Install(args) => hook::install(&args),
        },
        Command::Schema(args) => schema::run(&args),
        Command::Write(args) => runtime::block_on(write::run(args)),
        Command::Read(args) => runtime::block_on(read::run(args)),
        Command::Query(args) => runtime::block_on(query::run(args)),
        Command::Inject { action } => match action {
            InjectAction::Log(args) => inject::log(&args),
        },
    }
}
