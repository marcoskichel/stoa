//! `.stoa/recall.db` schema (FTS5 docs vtable + KG nodes/edges).
//!
//! Owned in Rust because the Bm25 backend owns the read+write hot path
//! and the schema needs to stay in lock-step with what the Python sidecar
//! upserts. Mirrors the `stoa-queue` `user_version` migration ladder so
//! adding a column is a one-line bump rather than a torn-DB migration.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::bm25::Bm25Error;

/// Default filename under `.stoa/`. Flat layout matches `queue.db`.
pub const RECALL_DB_FILE: &str = "recall.db";

/// Current schema version. Bumped whenever a migration step is added.
const USER_VERSION: i64 = 1;

const CREATE_DOCS: &str = "\
CREATE VIRTUAL TABLE IF NOT EXISTS docs USING fts5(
    doc_id UNINDEXED,
    kind UNINDEXED,
    source_path UNINDEXED,
    content,
    tokenize = \"porter unicode61\"
);";

const CREATE_NODES: &str = "\
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    attrs_json TEXT NOT NULL
) STRICT;";

const CREATE_EDGES: &str = "\
CREATE TABLE IF NOT EXISTS edges (
    src TEXT NOT NULL,
    dst TEXT NOT NULL,
    type TEXT NOT NULL,
    conf REAL NOT NULL,
    sources_json TEXT NOT NULL,
    PRIMARY KEY (src, dst, type)
) STRICT;";

const CREATE_EDGES_SRC_INDEX: &str = "CREATE INDEX IF NOT EXISTS edges_src ON edges(src, type);";

const CREATE_EDGES_DST_INDEX: &str = "CREATE INDEX IF NOT EXISTS edges_dst ON edges(dst, type);";

const PRAGMAS: &str = "\
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;";

/// Open `<workspace>/.stoa/recall.db` and ensure the schema is current.
///
/// Creates the parent directory if missing. Idempotent: repeated calls on
/// an already-current DB are a no-op. The DB path itself is refused if it
/// is a symlink so a hostile `.stoa/recall.db -> /tmp/elsewhere` cannot
/// redirect WAL/SHM siblings into the link target.
pub fn ensure_schema(path: &Path) -> Result<Connection, Bm25Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    refuse_symlink(path)?;
    let conn = Connection::open_with_flags(path, recall_db_open_flags())?;
    conn.execute_batch(PRAGMAS)?;
    apply_schema(&conn)?;
    Ok(conn)
}

/// Flags shared by every callsite opening `recall.db`.
pub(crate) fn recall_db_open_flags() -> OpenFlags {
    OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
}

/// Refuse the open if the DB path itself is a symlink.
///
/// We intentionally do NOT walk parent components: macOS roots every
/// temp dir at `/var/folders -> /private/var/folders`, and rejecting
/// any symlink anywhere in the path makes every tempdir-based test fail
/// without adding meaningful defense.
pub(crate) fn refuse_symlink(path: &Path) -> Result<(), Bm25Error> {
    if let Ok(meta) = std::fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(Bm25Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("recall.db `{}` is a symlink — refusing to open", path.display()),
        )));
    }
    Ok(())
}

fn apply_schema(conn: &Connection) -> Result<(), Bm25Error> {
    conn.execute_batch(CREATE_DOCS)?;
    conn.execute_batch(CREATE_NODES)?;
    conn.execute_batch(CREATE_EDGES)?;
    conn.execute_batch(CREATE_EDGES_SRC_INDEX)?;
    conn.execute_batch(CREATE_EDGES_DST_INDEX)?;
    set_user_version(conn, USER_VERSION)?;
    Ok(())
}

fn set_user_version(conn: &Connection, v: i64) -> Result<(), Bm25Error> {
    conn.execute_batch(&format!("PRAGMA user_version = {v};"))?;
    Ok(())
}
