"""Smoke test for the synthesis-id slug helper."""

from __future__ import annotations

from stoa_crystallize.cli import _default_page_id


def test_simple_question_slug():
    sid = _default_page_id("Why did we pick Redis for caching?")
    assert sid.startswith("syn-")
    assert "redis" in sid


def test_empty_question_falls_back():
    sid = _default_page_id("")
    assert sid == "syn-unknown"
