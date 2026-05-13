"""JSON-RPC dispatch for stoa-recalld.

Each handler takes a parsed params dict and returns a result dict (or
raises a `HandlerError` with a stable code + message). The server module
serializes the result envelope.
"""

from __future__ import annotations

from dataclasses import asdict
from typing import Any, cast

from stoa_recalld.store import MemPalaceUnavailable, Store, StoreError


class HandlerError(Exception):
    """Typed error with a stable wire code."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def handle(method: str, params: dict[str, Any], store: Store) -> dict[str, Any]:
    """Dispatch a single RPC. Returns the `result` body on success."""
    try:
        if method == "search":
            return _search(params, store)
        if method == "mine":
            return _mine(params, store)
        if method == "write_wiki":
            return _write_wiki(params, store)
        if method == "read_wiki":
            return _read_wiki(params, store)
        if method == "health":
            return _health(store)
    except StoreError as e:
        raise HandlerError("store_error", str(e)) from e
    except MemPalaceUnavailable as e:
        raise HandlerError("mempalace_unavailable", str(e)) from e
    raise HandlerError("unknown_method", f"unknown method: {method}")


def _search(params: dict[str, Any], store: Store) -> dict[str, Any]:
    query = _require_str(params, "query")
    top_k = _opt_int(params, "top_k", 5)
    raw_filters = cast("dict[str, Any]", params.get("filters") or {})
    filters: dict[str, str] = {str(k): str(v) for k, v in raw_filters.items()}
    hits = store.search(query, top_k, filters)
    return {"hits": [asdict(h) for h in hits]}


def _mine(params: dict[str, Any], store: Store) -> dict[str, Any]:
    source_file = _require_str(params, "source_file")
    drawer_ids = store.mine(source_file)
    return {"drawer_ids": drawer_ids}


def _write_wiki(params: dict[str, Any], store: Store) -> dict[str, Any]:
    page_id = _require_str(params, "page_id")
    frontmatter = cast("dict[str, Any]", params.get("frontmatter") or {})
    body = _require_str(params, "body")
    if not isinstance(frontmatter, dict):
        raise HandlerError("invalid_argument", "frontmatter must be an object")
    written = store.write_wiki(page_id, frontmatter, body)
    return {"path": written.path}


def _read_wiki(params: dict[str, Any], store: Store) -> dict[str, Any]:
    page_id = _require_str(params, "page_id")
    fm, body, path = store.read_wiki(page_id)
    return {"frontmatter": fm, "body": body, "path": path}


def _health(store: Store) -> dict[str, Any]:
    return {
        "status": "ok",
        "palace_path": str(store.config.palace_path),
        "mempalace_version": store.mempalace_version(),
    }


def _require_str(params: dict[str, Any], key: str) -> str:
    val = params.get(key)
    if not isinstance(val, str) or not val:
        raise HandlerError("invalid_argument", f"missing or empty `{key}`")
    return val


def _opt_int(params: dict[str, Any], key: str, default: int) -> int:
    val = params.get(key)
    if val is None:
        return default
    if not isinstance(val, int):
        raise HandlerError("invalid_argument", f"`{key}` must be an integer")
    return val
