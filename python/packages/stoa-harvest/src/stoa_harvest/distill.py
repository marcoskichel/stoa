"""LLM-driven distillation of MemPalace drawers into wiki pages.

The harvest pass takes a batch of verbatim drawers (transcript chunks
from MemPalace), asks an LLM to identify the entities and decisions
they encode, and emits structured wiki-page candidates ready for the
``write_wiki`` RPC.

This module is intentionally LLM-agnostic at the boundary: callers pass
in a [`Distiller`] that produces a list of [`PageDraft`] from a drawer
batch. The default implementation calls the Anthropic API; tests stub
it out via a recorded transcript.
"""
# pyright: reportAny=false, reportUnknownMemberType=false, reportUnknownArgumentType=false, reportUnknownVariableType=false, reportMissingTypeStubs=false

from __future__ import annotations

from dataclasses import dataclass, field
import datetime as _dt
import json
import os
from typing import Any, Protocol, cast


@dataclass(frozen=True)
class DrawerSnippet:
    """One verbatim text chunk from MemPalace."""

    drawer_id: str
    text: str
    source_file: str


@dataclass
class PageDraft:
    """Wiki page candidate emitted by a [`Distiller`]."""

    page_id: str
    frontmatter: dict[str, Any] = field(default_factory=dict)
    body: str = ""


class Distiller(Protocol):
    """Strategy protocol for batch distillation."""

    def distill(self, drawers: list[DrawerSnippet]) -> list[PageDraft]:
        """Return zero or more wiki-page candidates for the input drawers."""


_SYSTEM_PROMPT = (
    "You are Stoa's harvest worker. Your job is to read short conversation "
    "transcript chunks and identify the durable entities and decisions worth "
    "promoting into a knowledge wiki. Output strict JSON only: a list of "
    "objects shaped like "
    '{"page_id":"ent-<slug>","kind":"entity"|"concept","title":"...","summary":"...","relationships":[]}. '
    "Each `page_id` MUST be globally stable and kebab-case prefixed with "
    "`ent-` (entities) or `con-` (concepts). Skip anything ephemeral."
)


class AnthropicDistiller:
    """Default `Distiller` backed by the Anthropic Messages API."""

    def __init__(self, model: str = "claude-opus-4-7", api_key: str | None = None) -> None:
        self._model = model
        self._api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")

    def available(self) -> bool:
        """True when an Anthropic API key is set and the SDK imports cleanly."""
        if not self._api_key:
            return False
        try:
            import anthropic
        except ImportError:
            return False
        return True

    def distill(self, drawers: list[DrawerSnippet]) -> list[PageDraft]:
        if not drawers:
            return []
        if not self.available():
            return []
        import anthropic

        client = anthropic.Anthropic(api_key=self._api_key)
        prompt = self._build_user_prompt(drawers)
        msg = client.messages.create(
            model=self._model,
            max_tokens=2048,
            system=_SYSTEM_PROMPT,
            messages=[{"role": "user", "content": prompt}],
        )
        text = _extract_text(msg)
        return list(_parse_drafts(text))

    @staticmethod
    def _build_user_prompt(drawers: list[DrawerSnippet]) -> str:
        parts: list[str] = ["DRAWERS:"]
        for d in drawers:
            parts.append(f"--- drawer {d.drawer_id} (source: {d.source_file}) ---")
            parts.append(d.text)
        return "\n\n".join(parts)


def _extract_text(msg: Any) -> str:
    """Best-effort text extraction from an Anthropic Messages response."""
    content = cast("list[Any]", getattr(msg, "content", []) or [])
    out: list[str] = []
    for block in content:
        block_text = cast("str | None", getattr(block, "text", None))
        if isinstance(block_text, str):
            out.append(block_text)
    return "\n".join(out).strip()


def _parse_drafts(text: str) -> list[PageDraft]:
    """Parse the LLM's JSON list into [`PageDraft`] objects.

    Tolerant — accepts a bare JSON array, a code-fenced JSON block, or
    a JSON object with a top-level ``pages`` field. Anything else
    yields zero drafts so a malformed response degrades to a silent
    no-op rather than crashing the harvest worker.
    """
    raw = _strip_fence(text)
    try:
        decoded = cast("Any", json.loads(raw))
    except json.JSONDecodeError:
        return []
    if isinstance(decoded, dict) and "pages" in decoded:
        decoded = cast("Any", decoded["pages"])
    if not isinstance(decoded, list):
        return []
    drafts: list[PageDraft] = []
    for entry in cast("list[Any]", decoded):
        if isinstance(entry, dict):
            drafts.append(_draft_from_dict(cast("dict[str, Any]", entry)))
    return drafts


def _strip_fence(text: str) -> str:
    raw = text.strip()
    if raw.startswith("```"):
        raw = raw.strip("`")
        if raw.lower().startswith("json"):
            raw = raw[4:]
        raw = raw.strip()
    return raw


def _draft_from_dict(entry: dict[str, Any]) -> PageDraft:
    page_id = str(entry.get("page_id", "")).strip()
    kind = str(entry.get("kind", "entity")).strip() or "entity"
    title = str(entry.get("title", page_id)).strip() or page_id
    summary = str(entry.get("summary", "")).strip()
    rels_raw = cast("list[Any] | None", entry.get("relationships") or [])
    relationships: list[dict[str, Any]] = []
    if isinstance(rels_raw, list):
        for r in rels_raw:
            if isinstance(r, dict):
                relationships.append(cast("dict[str, Any]", r))
    now = _dt.datetime.now(_dt.UTC).isoformat()
    frontmatter: dict[str, Any] = {
        "id": page_id,
        "title": title,
        "kind": kind,
        "status": "active",
        "created": now,
        "updated": now,
    }
    if kind == "entity":
        frontmatter["type"] = str(entry.get("type", "concept"))
    if relationships:
        frontmatter["relationships"] = relationships
    body = summary or f"# {title}\n\n(no summary)"
    return PageDraft(page_id=page_id, frontmatter=frontmatter, body=body)
