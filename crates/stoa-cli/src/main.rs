//! Stoa CLI entry point.
//!
//! M2 — Wiki + CLI core. Subcommands live in sibling modules; this file is
//! a thin dispatch shell so per-command logic stays self-contained.

use std::process::ExitCode;

use clap::Parser;
use clap::error::ErrorKind;

mod catalog;
mod cli;
mod daemon;
mod hook;
mod index;
mod init;
mod page;
mod query;
mod read;
mod schema;
mod stoa_md;
mod workspace;
mod write;

use cli::Cli;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if wants_help(&argv) {
        return print_help();
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return handle_parse_error(&err),
    };
    run_or_fail(cli)
}

fn wants_help(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "--help" || a == "-h")
}

#[expect(
    clippy::print_stdout,
    reason = "`stoa --help` writes the help body to stdout by design."
)]
fn print_help() -> ExitCode {
    print!("{}", cli::HELP_BODY);
    ExitCode::SUCCESS
}

#[expect(clippy::print_stderr, reason = "Dispatch failures surface to stderr.")]
fn run_or_fail(cli: Cli) -> ExitCode {
    match cli.dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        },
    }
}

/// Clap-emitted errors take two paths: `--version` is an "error" that should
/// print to stdout + exit 0; everything else is a real error that the trycmd
/// snapshots expect to be terse (one line).
#[expect(
    clippy::print_stderr,
    reason = "Parse-error reporting writes the short message to stderr."
)]
fn handle_parse_error(err: &clap::Error) -> ExitCode {
    if matches!(err.kind(), ErrorKind::DisplayVersion) {
        let _ignored = err.print();
        return ExitCode::SUCCESS;
    }
    eprintln!("{}", short_parse_error(err));
    ExitCode::from(2)
}

fn short_parse_error(err: &clap::Error) -> String {
    let raw = err.to_string();
    raw.lines().next().unwrap_or("error").to_owned()
}
