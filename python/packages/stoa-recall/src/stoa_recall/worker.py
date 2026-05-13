"""Recall worker entry point.

Run as `python -m stoa_recall.worker --queue <path>`; loops claiming
rows from BOTH `recall.request` (write-side acks) AND `recall.search`
(read-side search), dispatching to a typed handler, and writing
`recall.response` rows back. ChromaDB / fastembed are not yet wired —
M4 ships the IPC round-trip + dispatch surface so the Rust IpcBackend
no longer always degrades to BM25; concrete vector / KG search lands
in M5.

The two-lane split exists so the Rust daemon can drain
`recall.request` (BM25 reindex on `index_page`/`remove`) without ever
seeing a `search` row it cannot service — which would otherwise cycle
claim → release indefinitely while the sidecar is offline.

Worker semantics mirror the Rust capture worker:

- One `BEGIN IMMEDIATE` transaction per claim.
- Lease semantics inherit from the queue schema (`status='claimed'`,
  `lease_expires`); a poison row is auto-released after the lease.
- The worker writes the response row in the same DB so the Rust caller
  sees it via `Queue::take_response_for`.
"""

from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
import time

from stoa_recall import dispatch
from stoa_recall.ipc import REQUEST_LANE, RESPONSE_LANE, SEARCH_LANE

logger: logging.Logger = logging.getLogger(__name__)

POLL_INTERVAL_SECS: float = 0.05
WORKER_ID_PREFIX: str = "stoa-recall-py"
LEASE_SECS: int = 60
BUSY_TIMEOUT_MS: int = 5000


class WorkerArgs:
    """Parsed CLI arguments for the worker."""

    def __init__(self, queue: str, *, once: bool) -> None:
        """Capture the parsed CLI args."""
        self.queue: str = queue
        self.once: bool = once


def parse_args(argv: list[str]) -> WorkerArgs:
    """Parse `--queue` + `--once` flags into a typed bag."""
    parser = argparse.ArgumentParser(prog="stoa_recall.worker")
    _ = parser.add_argument("--queue", type=str, required=True, help="Path to queue.db")
    _ = parser.add_argument(
        "--once",
        action="store_true",
        help="Drain one row and exit (test/CI hook).",
    )
    raw = parser.parse_args(argv)
    queue: str = str(raw.queue)  # type: ignore[reportAny]
    once: bool = bool(raw.once)  # type: ignore[reportAny]
    return WorkerArgs(queue=queue, once=once)


def main(argv: list[str] | None = None) -> int:
    """Worker entry point."""
    args = parse_args(argv if argv is not None else sys.argv[1:])
    logging.basicConfig(level=logging.INFO)
    logger.info("stoa_recall worker starting (queue=%s once=%s)", args.queue, args.once)
    if args.once:
        return _drain_once(args.queue)
    return _serve(args.queue)


def _drain_once(queue_path: str) -> int:
    """Drain exactly one row and exit. Used by tests + CI."""
    with _open_db(queue_path) as conn:
        _ = _claim_and_handle(conn)
    return 0


def _serve(queue_path: str) -> int:
    """Long-running poll loop with simple fixed interval."""
    while True:
        try:
            with _open_db(queue_path) as conn:
                handled = _claim_and_handle(conn)
        except sqlite3.Error:
            logger.exception("recall worker DB error; sleeping before retry")
            handled = False
        if not handled:
            time.sleep(POLL_INTERVAL_SECS)


def _open_db(path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(path, isolation_level=None)
    _ = conn.execute(f"PRAGMA busy_timeout = {BUSY_TIMEOUT_MS};")
    return conn


def _claim_and_handle(conn: sqlite3.Connection) -> bool:
    """Claim one `recall.request` row, dispatch, write the response row."""
    claimed = _claim_one(conn)
    if claimed is None:
        return False
    row_id, session_id, payload = claimed
    response_payload = dispatch.handle(payload)
    _write_response(conn, session_id, response_payload)
    _ = conn.execute("UPDATE queue_events SET status='done' WHERE id = ?;", (row_id,))
    return True


def _claim_one(conn: sqlite3.Connection) -> tuple[int, str, str] | None:
    """Atomically claim the next row from either request lane.

    Drains BOTH `recall.request` (write-side acks) AND `recall.search`
    (read-side search). The Rust daemon only claims `recall.request`,
    so search rows are guaranteed to land on this worker.

    Uses the same `BEGIN IMMEDIATE` round-trip as the Rust
    `stoa-capture` worker so the two pools share lease semantics.
    """
    worker_id = f"{WORKER_ID_PREFIX}-{int(time.time() * 1000)}"
    lease_expires = int(time.time()) + LEASE_SECS
    _ = conn.execute("BEGIN IMMEDIATE;")
    try:
        cursor = conn.execute(
            _CLAIM_SQL,
            (worker_id, lease_expires, REQUEST_LANE, SEARCH_LANE),
        )
        row: object = cursor.fetchone()  # type: ignore[reportAny]
        _ = conn.execute("COMMIT;")
    except sqlite3.Error:
        _ = conn.execute("ROLLBACK;")
        raise
    return _coerce_claim(row)


_CLAIM_COLUMN_COUNT: int = 3


def _coerce_claim(row: object) -> tuple[int, str, str] | None:
    """Narrow the untyped `sqlite3.Row` into a typed claim tuple."""
    if (
        row is None
        or not isinstance(row, tuple)  # type: ignore[reportUnnecessaryIsInstance]
        or len(row) != _CLAIM_COLUMN_COUNT  # type: ignore[reportUnknownArgumentType]
    ):
        return None
    row_id_raw, session_id_raw, payload_raw = row  # type: ignore[reportUnknownVariableType]
    if (
        not isinstance(row_id_raw, int)
        or not isinstance(session_id_raw, str)
        or not isinstance(payload_raw, str)
    ):
        return None
    return row_id_raw, session_id_raw, payload_raw


def _write_response(conn: sqlite3.Connection, session_id: str, payload: str) -> None:
    _ = conn.execute(
        _INSERT_RESPONSE_SQL,
        (RESPONSE_LANE, session_id, "recall.response", payload),
    )


_INSERT_RESPONSE_SQL: str = """
INSERT INTO queue_events (lane, session_id, event, payload, status, created_at)
     VALUES (?, ?, ?, ?, 'pending', unixepoch());
"""


_CLAIM_SQL: str = """
UPDATE queue_events
    SET status = 'claimed',
        claimed_by = ?1,
        claimed_at = unixepoch(),
        lease_expires = ?2
  WHERE id = (
        SELECT id FROM queue_events
         WHERE (status = 'pending'
             OR (status = 'claimed' AND lease_expires < unixepoch()))
           AND lane IN (?3, ?4)
         ORDER BY id ASC
         LIMIT 1
       )
 RETURNING id, session_id, payload;
"""


if __name__ == "__main__":
    raise SystemExit(main())
