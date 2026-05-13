"""Wire shapes for the Rust ↔ Python recall IPC protocol.

The Rust `IpcBackend` enqueues rows on `recall.request`; this module
defines the typed payload + response shapes the Python worker reads
back. Both sides serialize via JSON; pydantic enforces the contract on
the Python side.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


class RecallRequest(BaseModel):
    """Payload schema for `recall.request` queue rows.

    `method` mirrors the Rust trait method (`search`, `index_page`,
    `remove`, `health_check`). `args` is method-specific JSON; the
    worker deserializes on dispatch.
    """

    method: Literal["search", "index_page", "remove", "health_check"]
    args: dict[str, object] = Field(default_factory=dict)
    deadline_unix_ms: int = 0


class RecallError(BaseModel):
    """Error sub-record on a failed `recall.response` payload."""

    kind: str = "Other"
    msg: str = ""


class RecallResponse(BaseModel):
    """Payload schema for `recall.response` queue rows.

    `ok=True` carries a method-shaped `result`. `ok=False` carries an
    `error` describing the failure. The Rust side awaits the row keyed
    by `session_id == request_id`.
    """

    ok: bool
    result: dict[str, object] = Field(default_factory=dict)
    error: RecallError | None = None


REQUEST_LANE: str = "recall.request"
RESPONSE_LANE: str = "recall.response"
