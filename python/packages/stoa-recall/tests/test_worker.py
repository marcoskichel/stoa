"""Worker IPC round-trip + dispatch tests."""

from __future__ import annotations

import json
import sqlite3
from typing import TYPE_CHECKING

import pytest

from stoa_recall import dispatch, worker
from stoa_recall.ipc import (
    REQUEST_LANE,
    RESPONSE_LANE,
    FailureResponse,
    SearchResponse,
)

if TYPE_CHECKING:
    from pathlib import Path

SCHEMA: str = """
CREATE TABLE queue_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    claimed_by TEXT,
    claimed_at INTEGER,
    lease_expires INTEGER,
    created_at INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    error_kind TEXT
);
"""

INSERT_REQUEST_SQL: str = """
INSERT INTO queue_events (lane, session_id, event, payload, status, created_at)
     VALUES (?, ?, ?, ?, 'pending', unixepoch());
"""

SELECT_RESPONSE_SQL: str = """
SELECT payload FROM queue_events
 WHERE lane = ? AND session_id = ? AND status = 'pending';
"""


@pytest.fixture
def queue_db(tmp_path: Path) -> Path:
    db_path = tmp_path / "queue.db"
    conn = sqlite3.connect(db_path)
    _ = conn.executescript(SCHEMA)
    conn.commit()
    conn.close()
    return db_path


def _enqueue(db: Path, payload: dict[str, object], session_id: str) -> None:
    conn = sqlite3.connect(db)
    _ = conn.execute(
        INSERT_REQUEST_SQL,
        (REQUEST_LANE, session_id, "recall.search", json.dumps(payload)),
    )
    conn.commit()
    conn.close()


def _read_response_payload(db: Path, session_id: str) -> str | None:
    conn = sqlite3.connect(db)
    cursor = conn.execute(SELECT_RESPONSE_SQL, (RESPONSE_LANE, session_id))
    raw: object = cursor.fetchone()  # type: ignore[reportAny]
    conn.close()
    if raw is None or not isinstance(raw, tuple):  # type: ignore[reportUnnecessaryIsInstance]
        return None
    cell = raw[0]  # type: ignore[reportUnknownVariableType]
    if not isinstance(cell, str):
        return None
    return cell


def test_dispatch_search_returns_empty_hits() -> None:
    raw = json.dumps(
        {"method": "search", "args": {"query": "redis", "k": 5}, "deadline_unix_ms": 0}
    )
    parsed = SearchResponse.model_validate_json(dispatch.handle(raw))
    assert parsed.ok is True
    assert parsed.result.hits == []


def test_dispatch_invalid_method_yields_failure() -> None:
    raw = json.dumps({"method": "made-up", "args": {}})
    parsed = FailureResponse.model_validate_json(dispatch.handle(raw))
    assert parsed.ok is False
    assert "validation failed" in parsed.error.msg


def test_worker_drain_once_writes_response_row(queue_db: Path) -> None:
    _enqueue(
        queue_db,
        {"method": "search", "args": {"query": "redis", "k": 3}, "deadline_unix_ms": 0},
        "req-search-1",
    )
    rc = worker.main(["--queue", str(queue_db), "--once"])
    assert rc == 0
    raw_response = _read_response_payload(queue_db, "req-search-1")
    assert raw_response is not None
    parsed = SearchResponse.model_validate_json(raw_response)
    assert parsed.ok is True
    assert parsed.result.hits == []
