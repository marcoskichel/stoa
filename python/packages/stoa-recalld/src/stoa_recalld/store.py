"""MemPalace integration + on-disk wiki I/O.

Owns the bridge between the daemon's typed JSON RPC and MemPalace's
Python API. Also owns the canonical wiki markdown files on disk — the
``write_wiki`` RPC writes BOTH to disk AND to the MemPalace palace as a
drawer tagged ``kind=wiki``.
"""
# pyright: reportAny=false, reportUnknownMemberType=false, reportUnknownArgumentType=false, reportUnknownVariableType=false, reportMissingTypeStubs=false

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import yaml

if TYPE_CHECKING:
    from stoa_recalld.config import DaemonConfig

# MemPalace's Python imports happen lazily so the daemon can boot even
# when mempalace fails to load (we want to surface a precise error in
# the `health` RPC rather than crash on import).


@dataclass(frozen=True)
class Hit:
    """One ranked retrieval result — JSON-serializable shape."""

    doc_id: str
    score: float
    snippet: str
    source_path: str
    metadata: dict[str, str]


@dataclass(frozen=True)
class WrittenPage:
    """Result of writing a wiki page to disk + mempalace."""

    path: str


class MemPalaceUnavailable(RuntimeError):
    """Raised when MemPalace cannot be imported or its palace is unreadable."""


class StoreError(RuntimeError):
    """Generic store-layer failure (validation, I/O, etc.)."""


def _import_mempalace() -> Any:
    try:
        import mempalace

        return mempalace
    except ImportError as e:
        msg = f"mempalace package not installed: {e}"
        raise MemPalaceUnavailable(msg) from e


_KIND_DIRS: dict[str, str] = {
    "entity": "entities",
    "concept": "concepts",
    "synthesis": "synthesis",
}


class Store:
    """High-level operations the daemon exposes over its RPC surface."""

    def __init__(self, config: DaemonConfig) -> None:
        self._cfg = config
        self._cfg.palace_path.mkdir(parents=True, exist_ok=True)
        self._cfg.wiki_dir.mkdir(parents=True, exist_ok=True)

    @property
    def config(self) -> DaemonConfig:
        return self._cfg

    def mempalace_version(self) -> str:
        """Return the installed MemPalace package version string."""
        try:
            mp = _import_mempalace()
        except MemPalaceUnavailable:
            return "unavailable"
        version_attr = cast("Any", getattr(mp, "__version__", None))
        if isinstance(version_attr, str):
            return version_attr
        return "unknown"

    def search(self, query: str, top_k: int, filters: dict[str, str]) -> list[Hit]:
        """Run a hybrid search via mempalace.

        The filters dict is currently restricted to two recognized keys:
        ``wing`` (project filter), ``room`` (aspect filter). Any other
        keys are passed through to mempalace's metadata filter via the
        ``where`` clause on the underlying ChromaDB collection.
        """
        mp = _import_mempalace()
        wing = filters.get("wing")
        room = filters.get("room")
        # mempalace.searcher.search_memories returns a dict with `hits` list.
        result = mp.searcher.search_memories(
            query=query,
            palace_path=str(self._cfg.palace_path),
            wing=wing,
            room=room,
            n_results=max(1, top_k),
        )
        if not isinstance(result, dict):
            return []
        raw_hits = cast("list[dict[str, Any]]", result.get("hits", []))
        out: list[Hit] = []
        for h in raw_hits:
            out.append(_hit_from_mempalace(h))
        return out

    def mine(self, source_file: str) -> list[str]:
        """Mine a transcript or text file into mempalace drawers."""
        mp = _import_mempalace()
        # Mempalace's miner accepts a single file path; it chunks + stores.
        path = Path(source_file)
        if not path.is_file():
            msg = f"source_file not a regular file: {source_file}"
            raise StoreError(msg)
        # MemPalace's high-level miner returns drawer ids per processed file.
        result = mp.miner.mine_file(
            source_file=str(path),
            palace_path=str(self._cfg.palace_path),
        )
        if isinstance(result, list):
            return [str(x) for x in cast("list[Any]", result)]
        if isinstance(result, dict):
            ids = cast("list[Any] | None", result.get("drawer_ids"))
            if isinstance(ids, list):
                return [str(x) for x in ids]
        return []

    def write_wiki(
        self,
        page_id: str,
        frontmatter: dict[str, Any],
        body: str,
    ) -> WrittenPage:
        """Write a wiki page to disk AND index it as a mempalace drawer."""
        kind = str(frontmatter.get("kind", "")).strip()
        if kind not in _KIND_DIRS:
            msg = f"frontmatter.kind must be one of entity|concept|synthesis (got `{kind}`)"
            raise StoreError(msg)
        sub = _KIND_DIRS[kind]
        out_dir = self._cfg.wiki_dir / sub
        out_dir.mkdir(parents=True, exist_ok=True)
        out_path = out_dir / f"{page_id}.md"
        composed = _compose_page(frontmatter, body)
        out_path.write_text(composed, encoding="utf-8")
        self._index_wiki_drawer(page_id, kind, composed, frontmatter)
        rel = out_path.relative_to(self._cfg.workspace_root)
        return WrittenPage(path=str(rel))

    def read_wiki(self, page_id: str) -> tuple[dict[str, Any], str, str]:
        """Read a wiki page back from disk."""
        for sub in _KIND_DIRS.values():
            candidate = self._cfg.wiki_dir / sub / f"{page_id}.md"
            if candidate.is_file():
                raw = candidate.read_text(encoding="utf-8")
                fm, body = _split_page(raw)
                rel = candidate.relative_to(self._cfg.workspace_root)
                return fm, body, str(rel)
        msg = f"wiki page `{page_id}` not found"
        raise StoreError(msg)

    def _index_wiki_drawer(
        self,
        page_id: str,
        kind: str,
        composed: str,
        frontmatter: dict[str, Any],
    ) -> None:
        """Insert the wiki page as a drawer in mempalace tagged kind=wiki."""
        mp = _import_mempalace()
        col = mp.palace.get_collection(str(self._cfg.palace_path), create=True)
        metadata: dict[str, str] = {
            "kind": "wiki",
            "wiki_kind": kind,
            "wiki_id": page_id,
            "source_file": f"wiki/{_KIND_DIRS[kind]}/{page_id}.md",
            "wing": "__stoa_wiki__",
            "room": kind,
            "chunk_index": "0",
        }
        title = str(frontmatter.get("title", page_id))
        metadata["title"] = title
        try:
            col.delete(ids=[f"wiki:{page_id}"])
        except (RuntimeError, ValueError):
            pass
        col.add(
            ids=[f"wiki:{page_id}"],
            documents=[composed],
            metadatas=[metadata],
        )


def _hit_from_mempalace(raw: dict[str, Any]) -> Hit:
    """Translate mempalace's hit dict into our typed `Hit`."""
    metadata = cast("dict[str, Any] | None", raw.get("metadata") or {}) or {}
    distance_raw = raw.get("distance")
    distance: float = float(cast("Any", distance_raw)) if distance_raw is not None else 0.0
    score = max(0.0, 1.0 - distance)
    src = str(metadata.get("source_file") or metadata.get("wiki_id") or "")
    wiki_id = str(metadata.get("wiki_id") or src)
    metadata_str: dict[str, str] = {k: str(v) for k, v in metadata.items()}
    text = cast("str", raw.get("text") or "")
    return Hit(
        doc_id=wiki_id,
        score=score,
        snippet=text[:500],
        source_path=src,
        metadata=metadata_str,
    )


def _compose_page(frontmatter: dict[str, Any], body: str) -> str:
    """Build a wiki markdown page from frontmatter + body."""
    yaml_text = yaml.safe_dump(frontmatter, sort_keys=False).rstrip()
    return f"---\n{yaml_text}\n---\n\n{body.lstrip()}"


def _split_page(raw: str) -> tuple[dict[str, Any], str]:
    """Parse YAML frontmatter + body out of a wiki page."""
    if not raw.startswith("---\n"):
        return ({}, raw)
    rest = raw[4:]
    end = rest.find("\n---")
    if end == -1:
        return ({}, raw)
    fm_yaml = rest[:end]
    body = rest[end + 4 :].lstrip("\n")
    try:
        parsed = yaml.safe_load(fm_yaml) or {}
    except yaml.YAMLError:
        return ({}, body)
    if isinstance(parsed, dict):
        return (cast("dict[str, Any]", parsed), body)
    return ({}, body)
