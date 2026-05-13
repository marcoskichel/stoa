//! `stoa-inject-hook` binary — Claude Code `SessionStart` + `UserPromptSubmit` hook.
//!
//! Reads JSON payload on stdin, calls [`stoa_inject_hooks::run`] inside
//! a current-thread tokio runtime, writes `hookSpecificOutput` JSON to
//! stdout. Exit code is the structured signal; stderr is treated as a
//! warning by Claude Code (the host continues without an injection).

use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return report_io(&e),
    };
    match runtime.block_on(stoa_inject_hooks::run(io::stdin().lock(), io::stdout().lock())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(&e),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "Hook failure surfaces to stderr; exit code is the structured signal."
)]
fn report(err: &anyhow::Error) -> ExitCode {
    eprintln!("stoa-inject-hook: {err:#}");
    ExitCode::FAILURE
}

#[expect(
    clippy::print_stderr,
    reason = "Runtime construction failure surfaces to stderr."
)]
fn report_io(err: &io::Error) -> ExitCode {
    eprintln!("stoa-inject-hook: tokio runtime init failed: {err}");
    ExitCode::FAILURE
}
