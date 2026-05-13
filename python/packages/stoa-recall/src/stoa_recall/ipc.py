"""Wire shapes for the Rust ↔ Python recall IPC protocol.

The Rust `IpcBackend` enqueues rows on `recall.request`; this module
defines the typed payload + response shapes the Python worker reads
back. Both sides serialize via JSON; pydantic enforces the contract on
the Python side.

Per-method pydantic models (vs. the older `dict[str, object]` bag) so
basedpyright's `reportAny=error` cannot leak through. The
`RecallRequest` discriminated union dispatches on the `method`
literal — adding a new method requires adding a new model + wiring it
into the union.
"""

from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field


class StreamFilter(BaseModel):
    """Workspace-relative filters carried on `search` requests."""

    model_config = ConfigDict(extra="forbid")

    kind: str | None = None
    type_: str | None = Field(default=None, alias="type")


class SearchArgs(BaseModel):
    """Arguments for `recall.search`."""

    model_config = ConfigDict(extra="forbid")

    query: str
    k: int = 10
    streams: list[Literal["vector", "bm25", "graph"]] = Field(default_factory=list)
    filters: StreamFilter | None = None


class IndexPageArgs(BaseModel):
    """Arguments for `recall.index_page`.

    `path` is workspace-relative; the Python worker MUST refuse `..`
    and absolute paths (mirrors the Rust-side check in
    `crates/stoa-cli/src/daemon/recall_drain.rs`).
    """

    model_config = ConfigDict(extra="forbid")

    page_id: str
    path: str
    content: str | None = None


class RemoveArgs(BaseModel):
    """Arguments for `recall.remove`."""

    model_config = ConfigDict(extra="forbid")

    doc_id: str


class HealthCheckArgs(BaseModel):
    """No-op argument bag for `recall.health_check`."""

    model_config = ConfigDict(extra="forbid")


class _BaseRequest(BaseModel):
    """Shared envelope fields on every typed request."""

    model_config = ConfigDict(extra="forbid")

    deadline_unix_ms: int = 0


class SearchRequest(_BaseRequest):
    """Typed envelope for a `recall.search` request."""

    method: Literal["search"]
    args: SearchArgs


class IndexPageRequest(_BaseRequest):
    """Typed envelope for a `recall.index_page` request."""

    method: Literal["index_page"]
    args: IndexPageArgs


class RemoveRequest(_BaseRequest):
    """Typed envelope for a `recall.remove` request."""

    method: Literal["remove"]
    args: RemoveArgs


class HealthCheckRequest(_BaseRequest):
    """Typed envelope for a `recall.health_check` request."""

    method: Literal["health_check"]
    args: HealthCheckArgs = Field(default_factory=HealthCheckArgs)


RecallRequest = Annotated[
    SearchRequest | IndexPageRequest | RemoveRequest | HealthCheckRequest,
    Field(discriminator="method"),
]
"""Discriminated union over every supported request method.

Use `pydantic.TypeAdapter(RecallRequest).validate_json(raw)` to dispatch
without hand-rolling a `match` on the raw method string.
"""


class HitModel(BaseModel):
    """One ranked retrieval result; mirrors the Rust `Hit` struct."""

    model_config = ConfigDict(extra="forbid")

    doc_id: str
    score: float
    snippet: str
    source_path: str
    streams_matched: list[Literal["vector", "bm25", "graph"]]
    metadata: dict[str, str] = Field(default_factory=dict)


class SearchResult(BaseModel):
    """Result payload for `recall.search`."""

    model_config = ConfigDict(extra="forbid")

    hits: list[HitModel]


class IndexPageResult(BaseModel):
    """Result payload for `recall.index_page`."""

    model_config = ConfigDict(extra="forbid")

    indexed: int = 0


class RemoveResult(BaseModel):
    """Result payload for `recall.remove`."""

    model_config = ConfigDict(extra="forbid")

    removed: int = 0


class HealthCheckResult(BaseModel):
    """Result payload for `recall.health_check`."""

    model_config = ConfigDict(extra="forbid")

    backend: str = "local-chroma-sqlite"
    up: bool = True
    indexed_docs: int = 0


class RecallError(BaseModel):
    """Error sub-record on a failed `recall.response` payload."""

    model_config = ConfigDict(extra="forbid")

    kind: str = "Other"
    msg: str = ""


class SearchResponse(BaseModel):
    """`ok=True` envelope for a search response."""

    model_config = ConfigDict(extra="forbid")

    ok: Literal[True]
    result: SearchResult


class IndexPageResponse(BaseModel):
    """`ok=True` envelope for an index_page response."""

    model_config = ConfigDict(extra="forbid")

    ok: Literal[True]
    result: IndexPageResult


class RemoveResponse(BaseModel):
    """`ok=True` envelope for a remove response."""

    model_config = ConfigDict(extra="forbid")

    ok: Literal[True]
    result: RemoveResult


class HealthCheckResponse(BaseModel):
    """`ok=True` envelope for a health_check response."""

    model_config = ConfigDict(extra="forbid")

    ok: Literal[True]
    result: HealthCheckResult


class FailureResponse(BaseModel):
    """`ok=False` envelope carrying a typed error."""

    model_config = ConfigDict(extra="forbid")

    ok: Literal[False]
    error: RecallError


REQUEST_LANE: str = "recall.request"
RESPONSE_LANE: str = "recall.response"
