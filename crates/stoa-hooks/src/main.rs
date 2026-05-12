//! Stoa hook binary (Claude Code `Stop` / `SessionEnd` integration).
//!
//! Cold-start budget: <10 ms p95 — the binary opens `.stoa/queue.db`,
//! inserts one row, and exits. Anything heavier runs in workers.

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::SUCCESS
}
