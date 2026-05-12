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
pub(crate) const USER_VERSION: i64 = 1;

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
    error_kind TEXT
) STRICT;";

/// Partial unique index gating duplicate live rows by `session_id`.
///
/// Excludes `status='done'` and `status='failed'` so a session can be
/// re-captured after the previous row has been completed or dead-lettered.
const CREATE_UNIQUE_INDEX: &str = "\
CREATE UNIQUE INDEX IF NOT EXISTS queue_events_session_live \
    ON queue_events(session_id) \
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

/// Apply the schema to a freshly opened `Connection`.
///
/// Brings the DB forward from any prior `user_version` up to
/// [`USER_VERSION`]. Idempotent: re-running on an already-current DB is a
/// no-op.
pub(crate) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLE)?;
    migrate_to_v1(conn)?;
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
