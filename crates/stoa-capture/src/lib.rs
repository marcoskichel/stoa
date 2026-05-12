//! Stoa capture worker (transcript drain + PII redaction).
//!
//! See [ARCHITECTURE.md §7 "Capture pipeline (the hot path)"] for the
//! worker semantics and [ARCHITECTURE.md §10 "Redaction filter"] for the
//! pattern catalogue.
//!
//! The capture worker:
//! 1. Claims one row from `.stoa/queue.db` with a lease.
//! 2. Reads the source session `JSONL` referenced by the payload.
//! 3. Runs the [`Redactor`] line-by-line.
//! 4. Writes the redacted output to `sessions/<session_id>.jsonl`.
//! 5. Appends an entry to `.stoa/audit.log`.
//! 6. Marks the queue row done.

mod audit;
mod error;
mod patterns;
mod redactor;
mod worker;

pub use error::{Error, Result};
pub use redactor::Redactor;
pub use worker::{DrainResult, WorkerConfig, drain_once};
