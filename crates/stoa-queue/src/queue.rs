//! Public `Queue` handle: open, insert, claim, complete.
//!
//! One `Queue` wraps one `rusqlite::Connection`. Connections are not
//! `Sync`, so workers each open their own. The DB itself is shared via
//! `WAL` mode (see ARCHITECTURE.md §15).

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OpenFlags, params};
use serde_json::Value;

use crate::claim::{ClaimedRow, claim_in_tx};
use crate::error::Result;
use crate::{pragma, schema};

/// Owning handle around a `SQLite` connection backing the queue.
///
/// Cheap to open; the underlying file is shared across processes via `WAL`.
#[derive(Debug)]
pub struct Queue {
    conn: Mutex<Connection>,
}

/// A single queue row, as observed by [`Queue::peek_first_pending`].
#[derive(Debug, Clone)]
pub struct Row {
    /// Auto-increment row id.
    pub id: i64,
    /// Session-level idempotency key.
    pub session_id: String,
    /// Event name.
    pub event: String,
    /// JSON payload (string-encoded; caller decodes).
    pub payload: String,
}

impl Queue {
    /// Open (or create) the queue at `path`. Applies PRAGMAs + schema.
    pub fn open(path: &Path) -> Result<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        pragma::apply(&conn)?;
        schema::apply(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Insert one event. Idempotent on `session_id` while a prior row for
    /// the same session is still live (status != 'done').
    pub fn insert(&self, event: &str, session_id: &str, payload: &Value) -> Result<()> {
        let payload_str = serde_json::to_string(payload)?;
        with_conn(&self.conn, |c| {
            c.execute(INSERT_SQL, params![session_id, event, payload_str])?;
            Ok(())
        })
    }

    /// Atomically claim the next available row (pending OR expired-lease).
    pub fn claim(&self, worker_id: &str, lease_secs: i64) -> Result<Option<ClaimedRow>> {
        with_conn_mut(&self.conn, |c| {
            let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let row = claim_in_tx(&tx, worker_id, lease_secs)?;
            tx.commit()?;
            Ok(row)
        })
    }

    /// Mark a row done. Idempotent — running twice is a no-op.
    pub fn complete(&self, id: i64) -> Result<()> {
        with_conn(&self.conn, |c| {
            c.execute(COMPLETE_SQL, params![id])?;
            Ok(())
        })
    }

    /// Count rows whose status != 'done'.
    pub fn pending_count(&self) -> Result<u64> {
        with_conn(&self.conn, |c| {
            let n: i64 = c.query_row(PENDING_COUNT_SQL, [], |r| r.get(0))?;
            Ok(u64::try_from(n).unwrap_or(0))
        })
    }

    /// Peek the first pending row without claiming it. Test helper.
    pub fn peek_first_pending(&self) -> Result<Option<Row>> {
        with_conn(&self.conn, |c| {
            let mut stmt = c.prepare(PEEK_SQL)?;
            let mut rows = stmt.query([])?;
            let Some(r) = rows.next()? else {
                return Ok(None);
            };
            Ok(Some(Row {
                id: r.get(0)?,
                session_id: r.get(1)?,
                event: r.get(2)?,
                payload: r.get(3)?,
            }))
        })
    }

    /// `PRAGMA journal_mode` (lowercase string, e.g. `"wal"`).
    pub fn pragma_journal_mode(&self) -> Result<String> {
        with_conn(&self.conn, pragma::journal_mode)
    }

    /// `PRAGMA synchronous` as an integer (0..=3).
    pub fn pragma_synchronous(&self) -> Result<i64> {
        with_conn(&self.conn, pragma::synchronous)
    }

    /// `PRAGMA busy_timeout` in ms.
    pub fn pragma_busy_timeout(&self) -> Result<i64> {
        with_conn(&self.conn, pragma::busy_timeout)
    }
}

/// Run `f` while holding the connection lock; drop the guard before returning
/// so callers don't accidentally hold contention across awaits or further work.
fn with_conn<R>(mu: &Mutex<Connection>, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
    let guard = match mu.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let result = f(&guard);
    drop(guard);
    result
}

fn with_conn_mut<R>(
    mu: &Mutex<Connection>,
    f: impl FnOnce(&mut Connection) -> Result<R>,
) -> Result<R> {
    let mut guard = match mu.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let result = f(&mut guard);
    drop(guard);
    result
}

/// `INSERT OR IGNORE` honors the partial unique index, giving us
/// session-level idempotency without an extra round-trip.
const INSERT_SQL: &str = "\
INSERT OR IGNORE INTO queue_events \
    (session_id, event, payload, status, created_at) \
    VALUES (?1, ?2, ?3, 'pending', unixepoch());";

const COMPLETE_SQL: &str = "UPDATE queue_events SET status='done' WHERE id=?1;";

const PENDING_COUNT_SQL: &str = "SELECT COUNT(*) FROM queue_events WHERE status != 'done';";

const PEEK_SQL: &str = "\
SELECT id, session_id, event, payload \
  FROM queue_events \
 WHERE status != 'done' \
 ORDER BY id ASC \
 LIMIT 1;";
