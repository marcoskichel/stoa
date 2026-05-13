//! Stoa memory + recall benchmark runners.
//!
//! Hosts the v0.1 suite (`LongMemEval`, `MemoryAgentBench`, `MEMTRACK`, `BEAM`,
//! `AgentLeak`, `MTEB`-subset) and post-MVP runners. See `benchmarks/README.md`
//! for the full plan; per-benchmark intent + cost lives in
//! `benchmarks/<name>/README.md`.

mod adapter;
mod adapters;
mod backends;
mod cli;
mod error;
mod result;
mod run;

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match run::run(&cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&e);
            ExitCode::FAILURE
        },
    }
}

#[expect(
    clippy::print_stderr,
    reason = "CLI binary — errors must reach the terminal"
)]
fn report_error(e: &error::BenchError) {
    eprintln!("stoa-bench: {e}");
}
