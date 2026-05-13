"""Smoke tests for stoa-recall sidecar package."""

from __future__ import annotations

from pydantic import TypeAdapter

import stoa_recall
from stoa_recall.ipc import (
    REQUEST_LANE,
    RESPONSE_LANE,
    FailureResponse,
    IndexPageRequest,
    RecallError,
    RecallRequest,
    SearchArgs,
    SearchRequest,
)


def test_version_is_not_empty() -> None:
    assert stoa_recall.VERSION


def test_lane_names() -> None:
    assert REQUEST_LANE == "recall.request"
    assert RESPONSE_LANE == "recall.response"


def test_search_request_round_trips() -> None:
    payload = SearchRequest(method="search", args=SearchArgs(query="redis", k=5))
    raw = payload.model_dump_json()
    parsed = SearchRequest.model_validate_json(raw)
    assert parsed.method == "search"
    assert parsed.args.query == "redis"
    assert parsed.args.k == 5


def test_failure_response_carries_error() -> None:
    failure = FailureResponse(ok=False, error=RecallError(kind="Other", msg="boom"))
    raw = failure.model_dump_json()
    parsed = FailureResponse.model_validate_json(raw)
    assert parsed.ok is False
    assert parsed.error.msg == "boom"


def test_discriminated_union_dispatches_on_method() -> None:
    adapter: TypeAdapter[RecallRequest] = TypeAdapter(RecallRequest)
    raw = (
        '{"method":"index_page","args":{"page_id":"ent-redis","path":"wiki/entities/ent-redis.md"}}'
    )
    parsed = adapter.validate_json(raw)
    assert isinstance(parsed, IndexPageRequest)
    assert parsed.args.page_id == "ent-redis"
