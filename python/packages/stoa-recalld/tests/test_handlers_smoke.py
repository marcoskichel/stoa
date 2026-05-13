"""Smoke tests for the handler dispatch.

These tests stub the `Store` so they don't require a running mempalace
or ChromaDB. The goal is to verify the handler/error wiring stays
intact across refactors.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import pytest

from stoa_recalld.handlers import HandlerError, handle


@dataclass
class _FakeStore:
    palace_path_str: str = "/tmp/x"
    version: str = "test"

    @property
    def config(self):
        @dataclass
        class _C:
            palace_path: str

        return _C(palace_path=self.palace_path_str)

    def mempalace_version(self) -> str:
        return self.version

    def search(self, query: str, top_k: int, filters: dict[str, str]) -> list:
        return []

    def mine(self, source_file: str) -> list[str]:
        return ["d1"]

    def write_wiki(self, page_id: str, frontmatter: dict[str, Any], body: str):
        from stoa_recalld.store import WrittenPage

        return WrittenPage(path=f"wiki/entities/{page_id}.md")

    def read_wiki(self, page_id: str):
        return ({"id": page_id}, "body", f"wiki/entities/{page_id}.md")


def test_unknown_method():
    with pytest.raises(HandlerError) as exc:
        handle("nope", {}, _FakeStore())  # type: ignore[arg-type]
    assert exc.value.code == "unknown_method"


def test_health():
    out = handle("health", {}, _FakeStore())  # type: ignore[arg-type]
    assert out["status"] == "ok"
    assert out["mempalace_version"] == "test"


def test_search_empty():
    out = handle("search", {"query": "x", "top_k": 3}, _FakeStore())  # type: ignore[arg-type]
    assert out == {"hits": []}


def test_mine_returns_ids():
    out = handle("mine", {"source_file": "/tmp/x.jsonl"}, _FakeStore())  # type: ignore[arg-type]
    assert out == {"drawer_ids": ["d1"]}
