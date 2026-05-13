"""Pure-function dispatcher for `recall.request` payloads.

The worker (`worker.py`) calls `handle(raw_json)`; this module owns
the parsing + per-method response shape so the worker stays focused
on queue mechanics. Concrete vector / KG search lands in M5 — for
now `search` returns an empty hit list and the write paths return
trivial success acknowledgements.
"""

from __future__ import annotations

import json
import logging

from pydantic import TypeAdapter, ValidationError

from stoa_recall.ipc import (
    FailureResponse,
    HealthCheckRequest,
    HealthCheckResponse,
    HealthCheckResult,
    IndexPageRequest,
    IndexPageResponse,
    IndexPageResult,
    RecallError,
    RecallRequest,
    RemoveRequest,
    RemoveResponse,
    RemoveResult,
    SearchRequest,
    SearchResponse,
    SearchResult,
)

logger: logging.Logger = logging.getLogger(__name__)

_REQUEST_ADAPTER: TypeAdapter[RecallRequest] = TypeAdapter(RecallRequest)


def handle(raw_json: str) -> str:
    """Parse, dispatch, and serialize the response for one request.

    Always returns a JSON-serializable response payload — never raises.
    Validation failures and unsupported methods become `FailureResponse`.
    """
    try:
        request = _REQUEST_ADAPTER.validate_json(raw_json)
    except ValidationError as e:
        return _failure("InvalidArgument", f"validation failed: {e}")
    return _dispatch(request)


def _dispatch(request: RecallRequest) -> str:
    """Dispatch the parsed envelope to its per-method handler.

    The discriminated union is exhaustive at the type level — basedpyright
    narrows the final `else` branch to `HealthCheckRequest`.
    """
    match request:
        case SearchRequest():
            return _on_search(request)
        case IndexPageRequest():
            return _on_index_page(request)
        case RemoveRequest():
            return _on_remove(request)
        case HealthCheckRequest():
            return _on_health(request)


def _on_search(req: SearchRequest) -> str:
    """Empty-hits placeholder — real search lands in M5."""
    logger.info(
        "recall.search received (query=%s k=%d streams=%s); returning empty placeholder",
        req.args.query,
        req.args.k,
        ",".join(req.args.streams),
    )
    return SearchResponse(ok=True, result=SearchResult(hits=[])).model_dump_json()


def _on_index_page(req: IndexPageRequest) -> str:
    """Acknowledge — Rust daemon already handled BM25 indexing."""
    logger.info("recall.index_page acked (page_id=%s)", req.args.page_id)
    return IndexPageResponse(ok=True, result=IndexPageResult(indexed=0)).model_dump_json()


def _on_remove(req: RemoveRequest) -> str:
    """Acknowledge — Rust daemon already handled BM25 removal."""
    logger.info("recall.remove acked (doc_id=%s)", req.args.doc_id)
    return RemoveResponse(ok=True, result=RemoveResult(removed=0)).model_dump_json()


def _on_health(_req: HealthCheckRequest) -> str:
    """Static health response. Real `indexed_docs` count lands in M5."""
    return HealthCheckResponse(
        ok=True,
        result=HealthCheckResult(backend="local-chroma-sqlite", up=True, indexed_docs=0),
    ).model_dump_json()


def _failure(kind: str, msg: str) -> str:
    """Build a `FailureResponse` envelope as JSON."""
    payload = FailureResponse(ok=False, error=RecallError(kind=kind, msg=msg))
    return json.dumps(json.loads(payload.model_dump_json()))
