//! PRAGMA configuration applied on every `Connection::open`.
//!
//! WHY: `journal_mode=WAL` + `synchronous=NORMAL` give us low-latency
//! writes with crash safety adequate for a derived-state work queue (per
//! ARCHITECTURE §15). `temp_store=memory` keeps spill tables off disk;
//! `mmap_size=128MiB` makes reads cheap for small queues; `busy_timeout`
//! lets concurrent writers wait rather than fail immediately.

use rusqlite::Connection;

use crate::error::Result;

/// 128 MiB memory map — large enough for the queue to live entirely in
/// page cache on any modern machine.
const MMAP_BYTES: i64 = 128 * 1024 * 1024;

/// Busy timeout in ms (writer waits up to this long for a lock).
const BUSY_TIMEOUT_MS: i32 = 5000;

/// Apply all PRAGMAs in the order the `SQLite` docs recommend.
pub(crate) fn apply(conn: &Connection) -> Result<()> {
    set_journal_mode(conn)?;
    set_synchronous(conn)?;
    set_temp_store(conn)?;
    set_mmap_size(conn)?;
    set_busy_timeout(conn)?;
    Ok(())
}

fn set_journal_mode(conn: &Connection) -> Result<()> {
    let mode: String = conn.query_row("PRAGMA journal_mode=WAL;", [], |r| r.get(0))?;
    debug_assert_eq!(mode.to_ascii_lowercase(), "wal");
    Ok(())
}

fn set_synchronous(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
    Ok(())
}

fn set_temp_store(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA temp_store=MEMORY;")?;
    Ok(())
}

fn set_mmap_size(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!("PRAGMA mmap_size={MMAP_BYTES};"))?;
    Ok(())
}

fn set_busy_timeout(conn: &Connection) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_millis(
        u64::try_from(BUSY_TIMEOUT_MS).unwrap_or(5000),
    ))?;
    Ok(())
}

/// Read `PRAGMA journal_mode` for the test suite.
pub(crate) fn journal_mode(conn: &Connection) -> Result<String> {
    let mode: String = conn.query_row("PRAGMA journal_mode;", [], |r| r.get(0))?;
    Ok(mode)
}

/// Read `PRAGMA synchronous` as an integer (0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA).
pub(crate) fn synchronous(conn: &Connection) -> Result<i64> {
    let mode: i64 = conn.query_row("PRAGMA synchronous;", [], |r| r.get(0))?;
    Ok(mode)
}

/// Read `PRAGMA busy_timeout` in ms.
pub(crate) fn busy_timeout(conn: &Connection) -> Result<i64> {
    let ms: i64 = conn.query_row("PRAGMA busy_timeout;", [], |r| r.get(0))?;
    Ok(ms)
}
