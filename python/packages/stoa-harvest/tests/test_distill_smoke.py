"""Smoke tests for harvest distillation parsing.

These tests do not touch the Anthropic API or the daemon — they exercise
the JSON parser that turns model output into [`PageDraft`] objects.
"""

from __future__ import annotations

from stoa_harvest.distill import AnthropicDistiller, _parse_drafts


def test_parse_bare_json_array():
    text = (
        '[{"page_id":"ent-redis","kind":"entity","title":"Redis",'
        '"summary":"Cache","type":"library"}]'
    )
    drafts = _parse_drafts(text)
    assert len(drafts) == 1
    d = drafts[0]
    assert d.page_id == "ent-redis"
    assert d.frontmatter["title"] == "Redis"
    assert d.frontmatter["type"] == "library"


def test_parse_code_fenced_json():
    text = '```json\n[{"page_id":"con-rag","kind":"concept","title":"RAG","summary":"..."}]\n```'
    drafts = _parse_drafts(text)
    assert len(drafts) == 1
    assert drafts[0].page_id == "con-rag"


def test_parse_invalid_yields_empty():
    drafts = _parse_drafts("not json at all")
    assert drafts == []


def test_distiller_unavailable_without_key(monkeypatch):
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    d = AnthropicDistiller(api_key=None)
    assert not d.available()
