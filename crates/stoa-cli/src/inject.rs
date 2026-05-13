//! `stoa inject log` — print SessionStart injection history from
//! `.stoa/audit.log`.
//!
//! Spec: ARCHITECTURE.md §6.2 — every injection event is appended as a
//! single JSON line; this command tails the file (optionally filtered by
//! `--session` and capped by `--limit`).
//!
//! M5 skeleton: returns a "not implemented yet" error so the failing E2E
//! gates pin the contract before the impl agent fills it in.

use anyhow::anyhow;

/// Dispatched from `Cli::dispatch`.
pub(crate) fn log(_session: Option<&str>, _limit: Option<usize>) -> anyhow::Result<()> {
    Err(anyhow!("`stoa inject log` is not implemented yet (M5 skeleton)"))
}
