//! Stoa hook binary (Claude Code `Stop` / `SessionEnd` integration).
//!
//! Cold-start budget: <10 ms p95 — the binary opens `.stoa/queue.db`,
//! inserts one row, and exits. Anything heavier runs in workers.
//!
//! No `tokio`; no async runtime. The whole job is "one INSERT, exit."

use std::path::PathBuf;
use std::process::ExitCode;

use chrono::{SecondsFormat, Utc};
use clap::Parser;
use serde_json::json;

/// CLI args for the `stoa-hook` binary.
#[derive(Debug, Parser)]
#[command(name = "stoa-hook", version, about = "Stoa capture hook")]
struct Args {
    /// `.stoa/queue.db` path to insert into.
    #[arg(long)]
    queue: PathBuf,

    /// Session id (idempotency key — see ARCHITECTURE §7).
    #[arg(long)]
    session_id: String,

    /// Path to the raw session JSONL the worker will redact.
    #[arg(long)]
    session_path: String,

    /// Agent identifier (e.g. `claude-code`, `cursor`).
    #[arg(long)]
    agent_id: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(&e),
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let payload = json!({
        "session_id": args.session_id,
        "session_path": args.session_path,
        "agent_id": args.agent_id,
        "ts": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    });
    let q = stoa_queue::Queue::open(&args.queue)?;
    q.insert("agent.session.ended", &args.session_id, &payload)?;
    Ok(())
}

#[expect(
    clippy::print_stderr,
    reason = "Hook failure surfaces to stderr; exit code is the structured signal."
)]
fn report(err: &anyhow::Error) -> ExitCode {
    eprintln!("stoa-hook: {err:#}");
    ExitCode::FAILURE
}
