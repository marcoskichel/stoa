//! Atomic claim-with-lease primitive.
//!
//! Workers call `Queue::claim(worker_id, lease_secs)` which runs a single
//! `BEGIN IMMEDIATE` transaction selecting the lowest-id row whose status
//! is `pending`, or `claimed` with an expired lease, and updates it in
//! place. The `RETURNING` clause hands us the canonical view of the row
//! that was claimed.

use rusqlite::{OptionalExtension, Transaction, params};

use crate::error::Result;

/// A row that has been atomically claimed by a worker.
#[derive(Debug, Clone)]
pub struct ClaimedRow {
    /// Auto-increment row id.
    pub id: i64,
    /// The session-level idempotency key originally inserted.
    pub session_id: String,
    /// The event name (e.g. `agent.session.ended`).
    pub event: String,
    /// JSON-serialized payload (caller decodes).
    pub payload: String,
}

/// Run the claim primitive inside an open transaction. The caller is
/// expected to wrap this in `BEGIN IMMEDIATE` and commit on success.
pub(crate) fn claim_in_tx(
    tx: &Transaction<'_>,
    worker_id: &str,
    lease_secs: i64,
) -> Result<Option<ClaimedRow>> {
    let lease_expires = unix_now().saturating_add(lease_secs);
    let row = tx
        .query_row(CLAIM_SQL, params![worker_id, lease_expires], extract_row)
        .optional()?;
    Ok(row)
}

fn extract_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClaimedRow> {
    Ok(ClaimedRow {
        id: r.get(0)?,
        session_id: r.get(1)?,
        event: r.get(2)?,
        payload: r.get(3)?,
    })
}

/// `BEGIN IMMEDIATE`-friendly SQL that selects the next claimable row
/// and updates it in-place. The subquery picks the lowest-id row whose
/// status is `pending`, or `claimed` with an expired lease.
const CLAIM_SQL: &str = "\
UPDATE queue_events \
   SET status = 'claimed', \
       claimed_by = ?1, \
       claimed_at = unixepoch(), \
       lease_expires = ?2 \
 WHERE id = ( \
       SELECT id FROM queue_events \
        WHERE status = 'pending' \
           OR (status = 'claimed' AND lease_expires < unixepoch()) \
        ORDER BY id ASC \
        LIMIT 1 \
 ) \
RETURNING id, session_id, event, payload;";

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
