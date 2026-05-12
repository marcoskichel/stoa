// Minimal hook spike: open .stoa/queue.db (WAL, NORMAL), insert one row, exit.
// Mirrors the M3 contract: Stop/SessionEnd hook does one thing - enqueue a capture event.

use rusqlite::{Connection, params};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let db_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/stoa-spike-queue.db"));

    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("open: {e}");
            return ExitCode::from(2);
        }
    };

    if let Err(e) = conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS queue (
             id INTEGER PRIMARY KEY,
             session_id TEXT NOT NULL,
             event TEXT NOT NULL,
             payload BLOB NOT NULL,
             ts INTEGER NOT NULL
         );",
    ) {
        eprintln!("init: {e}");
        return ExitCode::from(3);
    }

    let session_id = args.get(2).cloned().unwrap_or_else(|| "spike".into());
    let event = "transcript.captured";
    let payload = b"{}";
    let ts: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    if let Err(e) = conn.execute(
        "INSERT INTO queue (session_id, event, payload, ts) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, event, payload, ts],
    ) {
        eprintln!("insert: {e}");
        return ExitCode::from(4);
    }

    ExitCode::SUCCESS
}
