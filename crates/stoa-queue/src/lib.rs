//! Stoa `SQLite` queue (`rusqlite`, WAL, claim-with-lease).
//!
//! See [ARCHITECTURE.md §7] for semantics and [ARCHITECTURE.md §15] for the
//! runtime choice (`rusqlite` v0.38, `WAL`, `synchronous=NORMAL`).
//!
//! The queue is a single `SQLite` table that drives the capture hot-path:
//! hooks insert one row per `agent.session.ended` event and exit; workers
//! claim rows with a lease, complete the work, and mark them done. The
//! schema is `STRICT` and has a partial unique index on `session_id` that
//! excludes `status='done'` so a session can be re-captured after its prior
//! row completes (idempotency on the live tail).

mod claim;
mod error;
mod pragma;
mod queue;
mod schema;

pub use claim::ClaimedRow;
pub use error::{Error, Result};
pub use queue::{FailureOutcome, Queue, Row};
