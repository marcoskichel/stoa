//! `stoa-doclint` — forbid non-doc comments outside an allowlist of intent
//! prefixes.
//!
//! Comments come in two roles:
//!
//! - **API documentation** — `///`, `//!`, `/** */`, `/*! */`. Always allowed.
//!   These survive `rustdoc` and live on alongside the items they describe.
//! - **Inline notes** — bare `//` and `/* */`. The lint forbids these unless
//!   the line comment opens with an explicit intent prefix from
//!   [`comments::ALLOWED_PREFIXES`]. The prefix forces the author to state
//!   *why* the comment exists.
//!
//! Trade-off: `TODO:` is deliberately excluded — TODOs decay into lies; track
//! them in the issue tracker so they have an owner and a state.
//!
//! Exit code is 0 if no findings, 1 otherwise. `--warn-only` always exits 0.

#![expect(
    clippy::print_stdout,
    reason = "CLI binary; stdout is the result channel"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod lint;

/// Forbid non-doc comments outside the prefix allowlist.
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
        println!("OK: no forbidden comments in {} root(s).", roots.len());
        ExitCode::SUCCESS
    } else if cli.warn_only {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
