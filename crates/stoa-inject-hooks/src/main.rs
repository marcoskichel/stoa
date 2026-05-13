//! `stoa-inject-hook` binary — Claude-Code SessionStart hook entrypoint.
//!
//! Reads the SessionStart JSON payload from stdin, calls
//! [`stoa_inject_hooks::run`], and writes the `hookSpecificOutput` JSON to
//! stdout. Non-zero exit on internal failure; the host treats stderr as a
//! warning and continues without an injection.

use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    match stoa_inject_hooks::run(io::stdin().lock(), io::stdout().lock()) {
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
