//! Stoa CLI entry point.

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
