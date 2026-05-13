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

/// Outcome of [`Queue::record_failure`].
#[derive(Debug, Clone, Copy)]
pub struct FailureOutcome {
    /// True if the row was moved to `status='failed'` (max attempts hit).
    pub dead_lettered: bool,
    /// Updated `attempts` count for the row.
    pub attempts: i64,
}

impl Queue {
    /// Open the queue at `path` on the fast path.
    ///
    /// Applies PRAGMAs, then reads `PRAGMA user_version`. If the DB is at
    /// the current [`schema::USER_VERSION`] this returns without touching
    /// the schema (the typical case for hooks). For new / migrating DBs
    /// the full [`schema::apply`] runs as a fallback so first-run callers
    /// (tests, fresh installs) are unaffected.
    ///
    /// Long-running callers (the daemon) should prefer [`Queue::init`]
    /// which is explicit about taking the schema path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = open_connection(path)?;
        pragma::apply(&conn)?;
        if !schema::is_current(&conn)? {
            schema::apply(&conn)?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open (or create) the queue at `path` and force a schema sync.
    ///
    /// Equivalent to [`Queue::open`] but always runs
    /// `CREATE TABLE IF NOT EXISTS` + index DDL + the migration sequence
    /// up to [`schema::USER_VERSION`]. Use this on daemon startup.
    pub fn init(path: &Path) -> Result<Self> {
        let conn = open_connection(path)?;
        pragma::apply(&conn)?;
        schema::apply(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Insert one event on the default `"capture"` lane.
    ///
    /// Idempotent on `(lane, session_id)` while a prior live row exists.
    /// Use [`Queue::insert_lane`] when targeting a non-default lane (e.g.
    /// M4 harvest's `"harvest"` lane).
    pub fn insert(&self, event: &str, session_id: &str, payload: &Value) -> Result<()> {
        self.insert_lane(schema::DEFAULT_LANE, event, session_id, payload)
    }

    /// Insert one event on the given lane. Idempotent on
    /// `(lane, session_id)` while a prior live row exists.
    pub fn insert_lane(
        &self,
        lane: &str,
        event: &str,
        session_id: &str,
        payload: &Value,
    ) -> Result<()> {
        let payload_str = serde_json::to_string(payload)?;
        with_conn(&self.conn, |c| {
            c.execute(INSERT_SQL, params![lane, session_id, event, payload_str])?;
            Ok(())
        })
    }

    /// Atomically claim the next available row across every lane.
    ///
    /// `worker_id` is recorded on the row for observability; `lease_secs`
    /// is the expiry budget before another worker may re-claim. Pending
    /// rows + claimed rows whose lease has expired are both eligible.
    pub fn claim(&self, worker_id: &str, lease_secs: i64) -> Result<Option<ClaimedRow>> {
        self.claim_on_lanes(worker_id, lease_secs, &[])
    }

    /// Claim the next available row restricted to one of `lanes`.
    ///
    /// An empty slice means "every lane" (equivalent to [`Queue::claim`]).
    /// Workers should pass their canonical lane name (`"capture"` for M3,
    /// `"harvest"` for M4) so they never grab rows targeted at a different
    /// worker pool.
    pub fn claim_on_lanes(
        &self,
        worker_id: &str,
        lease_secs: i64,
        lanes: &[&str],
    ) -> Result<Option<ClaimedRow>> {
        with_conn_mut(&self.conn, |c| {
            let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let row = claim_in_tx(&tx, worker_id, lease_secs, lanes)?;
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

    /// Record a failed processing attempt for row `id`.
    ///
    /// Increments `attempts` by 1. If the new `attempts` reaches
    /// `max_attempts`, the row is dead-lettered (`status='failed'`,
    /// `error_kind` set). Otherwise the row is released back to `pending`
    /// so the next worker tick can re-claim it.
    pub fn record_failure(
        &self,
        id: i64,
        error_kind: &str,
        max_attempts: i64,
    ) -> Result<FailureOutcome> {
        with_conn(&self.conn, |c| {
            let mut stmt = c.prepare(RECORD_FAILURE_SQL)?;
            let row = stmt.query_row(params![max_attempts, error_kind, id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            Ok(FailureOutcome {
                dead_lettered: row.0 == "failed",
                attempts: row.1,
            })
        })
    }

    /// Count rows whose status is `'pending'` or `'claimed'` (i.e. live work).
    pub fn pending_count(&self) -> Result<u64> {
        with_conn(&self.conn, |c| {
            let n: i64 = c.query_row(PENDING_COUNT_SQL, [], |r| r.get(0))?;
            Ok(u64::try_from(n).unwrap_or(0))
        })
    }

    /// Count rows whose status is `'failed'` (dead-lettered).
    pub fn failed_count(&self) -> Result<u64> {
        with_conn(&self.conn, |c| {
            let n: i64 = c.query_row(FAILED_COUNT_SQL, [], |r| r.get(0))?;
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

    /// Peek the first pending row on `lane` without claiming it. Test helper.
    pub fn peek_first_pending_on_lane(&self, lane: &str) -> Result<Option<Row>> {
        with_conn(&self.conn, |c| {
            let mut stmt = c.prepare(PEEK_LANE_SQL)?;
            let mut rows = stmt.query(params![lane])?;
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

    /// Run `PRAGMA wal_checkpoint(TRUNCATE)` to flush + truncate the WAL.
    ///
    /// `SQLite` checkpoints opportunistically but never truncates the WAL
    /// on its own; long-running daemons should call this periodically (e.g.
    /// after a stretch of idle backoff) to keep `.stoa/queue.db-wal` from
    /// growing unbounded.
    pub fn checkpoint(&self) -> Result<()> {
        with_conn(&self.conn, |c| {
            let mut stmt = c.prepare("PRAGMA wal_checkpoint(TRUNCATE);")?;
            let _ignored: (i64, i64, i64) =
                stmt.query_row([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            Ok(())
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

fn open_connection(path: &Path) -> Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags)?;
    Ok(conn)
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
/// `(lane, session_id)`-level idempotency without an extra round-trip.
const INSERT_SQL: &str = "\
INSERT OR IGNORE INTO queue_events \
    (lane, session_id, event, payload, status, created_at) \
    VALUES (?1, ?2, ?3, ?4, 'pending', unixepoch());";

const COMPLETE_SQL: &str = "UPDATE queue_events SET status='done' WHERE id=?1;";

/// Atomic failure-record statement.
///
/// `?1` = `max_attempts`, `?2` = `error_kind`, `?3` = row id. Increments
/// `attempts` by 1; if the new value reaches `max_attempts` the row is
/// dead-lettered (`status='failed'`, `error_kind` set), otherwise it is
/// released back to `pending` so the next claim picks it up.
const RECORD_FAILURE_SQL: &str = "\
UPDATE queue_events \
   SET attempts = attempts + 1, \
       status = CASE WHEN attempts + 1 >= ?1 THEN 'failed' ELSE 'pending' END, \
       error_kind = CASE WHEN attempts + 1 >= ?1 THEN ?2 ELSE error_kind END, \
       claimed_by = NULL, \
       claimed_at = NULL, \
       lease_expires = NULL \
 WHERE id = ?3 \
RETURNING status, attempts;";

const PENDING_COUNT_SQL: &str =
    "SELECT COUNT(*) FROM queue_events WHERE status IN ('pending', 'claimed');";

const FAILED_COUNT_SQL: &str = "SELECT COUNT(*) FROM queue_events WHERE status = 'failed';";

const PEEK_SQL: &str = "\
SELECT id, session_id, event, payload \
  FROM queue_events \
 WHERE status IN ('pending', 'claimed') \
 ORDER BY id ASC \
 LIMIT 1;";

const PEEK_LANE_SQL: &str = "\
SELECT id, session_id, event, payload \
  FROM queue_events \
 WHERE status IN ('pending', 'claimed') AND lane = ?1 \
 ORDER BY id ASC \
 LIMIT 1;";
