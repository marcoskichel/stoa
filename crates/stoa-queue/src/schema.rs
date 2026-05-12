//! `SQLite` schema and migrations for the queue.
//!
//! Single STRICT table `queue_events` + a partial unique index on
//! `session_id` that excludes `status='done'`. The partial index is what
//! gives us idempotency on the live tail: re-firing the same `session_id`
//! while the previous row is still `pending` or `claimed` is a no-op
//! (`INSERT OR IGNORE`), but a fresh insert succeeds once the prior row
//! has completed.

use rusqlite::Connection;

use crate::error::Result;

/// `CREATE TABLE IF NOT EXISTS` for the events queue.
///
/// All columns are explicit + `STRICT` so typos surface immediately.
const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS queue_events (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    event TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    claimed_by TEXT,
    claimed_at INTEGER,
    lease_expires INTEGER
) STRICT;";

/// Partial unique index gating duplicate live rows by `session_id`.
///
/// Excludes `status='done'` so a session can be re-captured after the
/// previous row has been completed.
const CREATE_UNIQUE_INDEX: &str = "\
CREATE UNIQUE INDEX IF NOT EXISTS queue_events_session_live \
    ON queue_events(session_id) \
    WHERE status != 'done';";

/// Secondary index for the claim hot-path (status + `lease_expires`).
const CREATE_STATUS_INDEX: &str = "\
CREATE INDEX IF NOT EXISTS queue_events_status_lease \
    ON queue_events(status, lease_expires);";

/// Apply the schema to a freshly opened `Connection`.
pub(crate) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLE)?;
    conn.execute_batch(CREATE_UNIQUE_INDEX)?;
    conn.execute_batch(CREATE_STATUS_INDEX)?;
    Ok(())
}
