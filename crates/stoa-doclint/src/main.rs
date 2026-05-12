//! `stoa-doclint` — flags `///` doc comments that restate the identifier.
//!
//! Heuristic-based, AST-only (no type info needed). See `lint.rs` for the rule.
//! Exit code 0 if clean, 1 if any findings (suppress with `--warn-only`).

#![expect(
    clippy::print_stdout,
    reason = "CLI binary; stdout is the result channel"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod lint;

/// Flag doc comments that add no information beyond the identifier.
#[derive(Parser, Debug)]
#[command(name = "stoa-doclint", version, about, long_about = None)]
struct Cli {
    /// Paths to scan recursively. Defaults to `crates`.
    paths: Vec<PathBuf>,
    /// Print findings but exit 0 even if any were found.
    #[arg(long)]
    warn_only: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let roots = if cli.paths.is_empty() {
        vec![PathBuf::from("crates")]
    } else {
        cli.paths
    };
    let findings = lint::run(&roots);
    for f in &findings {
        println!("{f}");
    }
    if findings.is_empty() {
        println!("OK: no trivial doc comments found in {} root(s).", roots.len());
        ExitCode::SUCCESS
    } else if cli.warn_only {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
