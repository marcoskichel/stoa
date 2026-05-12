//! Atomic claim-with-lease primitive.
//!
//! Workers call `Queue::claim(worker_id, lease_secs)` which runs a single
//! `BEGIN IMMEDIATE` transaction selecting the lowest-id row whose status
//! is `pending`, or `claimed` with an expired lease, and updates it in
//! place. The `RETURNING` clause hands us the canonical view of the row
//! that was claimed.

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

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
///
/// An empty `lanes` slice matches every lane; a non-empty slice restricts
/// the select to rows whose `lane` is one of the supplied values.
pub(crate) fn claim_in_tx(
    tx: &Transaction<'_>,
    worker_id: &str,
    lease_secs: i64,
    lanes: &[&str],
) -> Result<Option<ClaimedRow>> {
    let lease_expires = unix_now().saturating_add(lease_secs);
    let sql = build_claim_sql(lanes.len());
    let mut bindings: Vec<Value> = Vec::with_capacity(2 + lanes.len());
    bindings.push(Value::Text(worker_id.to_owned()));
    bindings.push(Value::Integer(lease_expires));
    for lane in lanes {
        bindings.push(Value::Text((*lane).to_owned()));
    }
    let row = tx
        .query_row(&sql, params_from_iter(bindings.iter()), extract_row)
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

/// Build the `BEGIN IMMEDIATE`-friendly SQL that selects the next
/// claimable row and updates it in-place.
///
/// When `lane_count == 0` the inner select considers every lane; otherwise
/// it adds `AND lane IN (?, ?, ...)` with one placeholder per slot. The
/// first two bind placeholders are always `worker_id` + `lease_expires`.
fn build_claim_sql(lane_count: usize) -> String {
    let mut s = String::from(CLAIM_SQL_PREFIX);
    if lane_count > 0 {
        append_lane_filter(&mut s, lane_count);
    }
    s.push_str(CLAIM_SQL_SUFFIX);
    s
}

fn append_lane_filter(s: &mut String, lane_count: usize) {
    s.push_str(" AND lane IN (");
    for i in 0..lane_count {
        if i > 0 {
            s.push(',');
        }
        s.push('?');
        s.push_str(&(i + 3).to_string());
    }
    s.push(')');
}

const CLAIM_SQL_PREFIX: &str = "\
UPDATE queue_events \
    SET status = 'claimed', \
        claimed_by = ?1, \
        claimed_at = unixepoch(), \
        lease_expires = ?2 \
  WHERE id = ( \
        SELECT id FROM queue_events \
         WHERE (status = 'pending' \
             OR (status = 'claimed' AND lease_expires < unixepoch()))";

const CLAIM_SQL_SUFFIX: &str = " ORDER BY id ASC \
              LIMIT 1 \
            ) \
        RETURNING id, session_id, event, payload;";

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
