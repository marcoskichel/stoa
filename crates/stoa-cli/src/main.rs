//! `stoa` CLI binary — orchestrates the MemPalace-backed pivot stack.
//!
//! Every retrieval / mining / wiki-write subcommand forwards to the
//! long-lived `stoa-recalld` Python daemon over its Unix socket. The
//! CLI owns: workspace scaffolding (`init`), daemon lifecycle (`daemon
//! start|stop|status`), Claude Code hook installation (`hook install`),
//! schema operations (`schema`), wiki I/O (`write`, `read`, `query`),
//! and audit-log inspection (`inject log`).

#![doc(html_no_source)]

mod cli;
mod commands;

use std::process::ExitCode;

use cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match commands::dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(&e),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "User-facing CLI errors go to stderr; exit code is the structured signal."
)]
fn report(err: &anyhow::Error) -> ExitCode {
    eprintln!("stoa: {err:#}");
    ExitCode::FAILURE
}
