//! Stoa CLI entry point.
//!
//! M1 skeleton — concrete commands land in M2 (Wiki + CLI core).

use std::process::ExitCode;

use clap::Parser;

/// Open-core knowledge + memory system for AI agents.
#[derive(Parser, Debug)]
#[command(name = "stoa", version, about, long_about = None)]
struct Cli {}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::SUCCESS
}
