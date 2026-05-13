//! Shared helpers for `stoa-capture` integration tests.

#![expect(
    dead_code,
    reason = "Helpers shared across test files; not all are used in every test binary."
)]
#![expect(
    unreachable_pub,
    reason = "`tests/common/` is included via `mod common;` per integration-test binary; `pub` is needed for cross-file sharing."
)]

use stoa_capture::Redactor;

/// Build a `Redactor` with default patterns (per ARCHITECTURE §10).
pub fn default_redactor() -> Redactor {
    Redactor::with_defaults()
}

/// Returns true if `s` contains a `[REDACTED:<kind>]` marker.
pub fn has_redaction_marker(s: &str) -> bool {
    s.contains("[REDACTED:")
}

/// Returns true if `s` contains a `[REDACTED:<kind>]` marker whose `<kind>`
/// matches `kind` (case-insensitive substring match).
pub fn has_redaction_kind(s: &str, kind: &str) -> bool {
    let needle = format!("[REDACTED:{kind}");
    s.contains(&needle)
}

/// Placeholder helper to keep the file-level `dead_code` expectation
/// fulfilled in every test binary — `redaction.rs` uses all three helpers
/// above, so without an always-unused fourth, the expect goes vacuous.
pub fn dead_code_sentinel() {}
