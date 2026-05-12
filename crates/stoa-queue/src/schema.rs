//! `SQLite` schema and migrations for the queue.
//!
//! Single STRICT table `queue_events` + a partial unique index on
//! `session_id` that excludes `status='done'`. The partial index is what
//! gives us idempotency on the live tail: re-firing the same `session_id`
//! while the previous row is still `pending` or `claimed` is a no-op
//! (`INSERT OR IGNORE`), but a fresh insert succeeds once the prior row
//! has completed.
//!
//! Schema evolution is tracked via `PRAGMA user_version`. The constant
//! [`USER_VERSION`] is the current target; [`apply`] is idempotent and
//! brings older DBs forward via `ALTER TABLE ADD COLUMN`.

use rusqlite::Connection;

use crate::error::Result;

/// Current schema version. Bumped whenever a migration step is added.
pub(crate) const USER_VERSION: i64 = 2;

/// Default lane used by M3 capture inserts. M4 harvest will introduce
/// additional lanes (e.g. `"harvest"`) without conflicting on the partial
/// unique index, since the index is keyed by `(lane, session_id)`.
pub(crate) const DEFAULT_LANE: &str = "capture";

/// `CREATE TABLE IF NOT EXISTS` for the events queue.
///
/// All columns are explicit + `STRICT` so typos surface immediately. Newer
/// columns (`attempts`, `error_kind`) carry safe defaults so older rows
/// migrated via `ADD COLUMN` are well-defined.
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
    lease_expires INTEGER,
    attempts INTEGER NOT NULL DEFAULT 0,
    error_kind TEXT,
    lane TEXT NOT NULL DEFAULT 'capture'
) STRICT;";

/// Partial unique index gating duplicate live rows by `(lane, session_id)`.
///
/// Lane-aware so M4 harvest's `transcript.captured` event for a given
/// session id can coexist with the M3 capture row. Excludes `done` +
/// `failed` so a session can be re-enqueued after the previous row has
/// been completed or dead-lettered.
const CREATE_UNIQUE_INDEX: &str = "\
CREATE UNIQUE INDEX IF NOT EXISTS queue_events_session_live \
    ON queue_events(lane, session_id) \
    WHERE status NOT IN ('done', 'failed');";

/// Secondary index for the claim hot-path (status + `lease_expires`).
const CREATE_STATUS_INDEX: &str = "\
CREATE INDEX IF NOT EXISTS queue_events_status_lease \
    ON queue_events(status, lease_expires);";

/// `ADD COLUMN` for `attempts` (idempotent via try/check pattern below).
const ADD_ATTEMPTS_SQL: &str =
    "ALTER TABLE queue_events ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;";

/// `ADD COLUMN` for `error_kind`.
const ADD_ERROR_KIND_SQL: &str = "ALTER TABLE queue_events ADD COLUMN error_kind TEXT;";

/// `ADD COLUMN` for `lane` (with default so older rows migrate cleanly).
const ADD_LANE_SQL: &str =
    "ALTER TABLE queue_events ADD COLUMN lane TEXT NOT NULL DEFAULT 'capture';";

/// Drop the legacy `(session_id)`-only partial index. Replaced by
/// `queue_events_session_live` on `(lane, session_id)` in [`apply`].
const DROP_LEGACY_INDEX_SQL: &str = "DROP INDEX IF EXISTS queue_events_session_live;";

/// Fast-path probe: returns true when `PRAGMA user_version` already
/// matches [`USER_VERSION`]. Callers (the hook hot-path) skip the
/// `CREATE TABLE IF NOT EXISTS` + index DDL when this is true.
pub(crate) fn is_current(conn: &Connection) -> Result<bool> {
    Ok(read_user_version(conn)? == USER_VERSION)
}

/// Apply the schema to a freshly opened `Connection`.
///
/// Brings the DB forward from any prior `user_version` up to
/// [`USER_VERSION`]. Idempotent: re-running on an already-current DB is a
/// no-op.
pub(crate) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLE)?;
    migrate_to_v1(conn)?;
    migrate_to_v2(conn)?;
    conn.execute_batch(CREATE_UNIQUE_INDEX)?;
    conn.execute_batch(CREATE_STATUS_INDEX)?;
    set_user_version(conn, USER_VERSION)?;
    Ok(())
}

fn migrate_to_v1(conn: &Connection) -> Result<()> {
    if read_user_version(conn)? >= 1 {
        return Ok(());
    }
    if !column_exists(conn, "attempts")? {
        conn.execute_batch(ADD_ATTEMPTS_SQL)?;
    }
    if !column_exists(conn, "error_kind")? {
        conn.execute_batch(ADD_ERROR_KIND_SQL)?;
    }
    Ok(())
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    if read_user_version(conn)? >= 2 {
        return Ok(());
    }
    if !column_exists(conn, "lane")? {
        conn.execute_batch(ADD_LANE_SQL)?;
    }
    conn.execute_batch(DROP_LEGACY_INDEX_SQL)?;
    Ok(())
}

fn read_user_version(conn: &Connection) -> Result<i64> {
    let v: i64 = conn.query_row("PRAGMA user_version;", [], |r| r.get(0))?;
    Ok(v)
}

fn set_user_version(conn: &Connection, v: i64) -> Result<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {v};"))?;
    Ok(())
}

fn column_exists(conn: &Connection, name: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info('queue_events');")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for row in rows {
        if row? == name {
            return Ok(true);
        }
    }
    Ok(false)
}
