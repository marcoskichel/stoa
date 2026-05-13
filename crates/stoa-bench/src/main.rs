//! Stoa memory + recall benchmark runners.
//!
//! Hosts the v0.1 suite and post-MVP runners. See `benchmarks/README.md`
//! for the full plan; per-benchmark intent + cost lives in
//! `benchmarks/<name>/README.md`.
//!
//! M4 ships the `LongMemEval` surface — `--dry-run` is the CI gate (no
//! dataset required); a real run reads `benchmarks/longmemeval/data/`
//! (gitignored) and writes scored hypotheses.

mod longmemeval;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Stoa benchmark runner.
#[derive(Parser, Debug)]
#[command(name = "stoa-bench", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// `LongMemEval` recall@k benchmark.
    Longmemeval(longmemeval::Args),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report_error(&err);
            ExitCode::FAILURE
        },
    }
}

fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Longmemeval(args) => longmemeval::run(&args),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "CLI surfaces dispatch failures via stderr."
)]
fn report_error(err: &anyhow::Error) {
    eprintln!("error: {err:#}");
}
