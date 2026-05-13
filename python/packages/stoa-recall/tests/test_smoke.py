"""Smoke tests for stoa-recall sidecar package."""

from __future__ import annotations

import stoa_recall
from stoa_recall.ipc import REQUEST_LANE, RESPONSE_LANE, RecallError, RecallRequest, RecallResponse


def test_version_is_not_empty() -> None:
    assert stoa_recall.VERSION


def test_lane_names() -> None:
    assert REQUEST_LANE == "recall.request"
    assert RESPONSE_LANE == "recall.response"


def test_request_round_trips() -> None:
    payload = RecallRequest(method="search", args={"query": "redis", "k": 5})
    raw = payload.model_dump_json()
    parsed = RecallRequest.model_validate_json(raw)
    assert parsed.method == "search"
    assert parsed.args["query"] == "redis"


def test_response_carries_error() -> None:
    failure = RecallResponse(ok=False, error=RecallError(kind="Other", msg="boom"))
    raw = failure.model_dump_json()
    parsed = RecallResponse.model_validate_json(raw)
    assert parsed.ok is False
    assert parsed.error is not None
    assert parsed.error.msg == "boom"
