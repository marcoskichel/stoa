"""`stoa-crystallize` CLI entry point.

Reads a set of wiki pages identified by a query, produces one
synthesis page that answers a high-level question across them, and
writes the result back via the daemon.

Reuses the [`AnthropicDistiller`][stoa_harvest.distill.AnthropicDistiller]
plumbing from `stoa-harvest` — the only differences are the prompt and
the resulting page's `kind: synthesis`.
"""
# pyright: reportAny=false, reportUnknownMemberType=false, reportUnknownArgumentType=false, reportUnknownVariableType=false

from __future__ import annotations

import argparse
import datetime as _dt
import os
import sys
from typing import Any, cast

from stoa_harvest.client import DaemonError, rpc


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="stoa-crystallize", description="Stoa LLM crystallize worker."
    )
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="Produce a synthesis page across recent wiki entries.")
    _ = run.add_argument("question", help="Question the synthesis should answer")
    _ = run.add_argument("--top-k", type=int, default=8, help="Wiki pages to consult")
    _ = run.add_argument("--model", default="claude-opus-4-7", help="Anthropic model id")
    _ = run.add_argument("--page-id", default="", help="Override the synthesis page id")
    _ = run.add_argument("--dry-run", action="store_true", help="Print result; do not write")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    if args.command == "run":
        return _cmd_run(args)
    return 2


def _cmd_run(args: argparse.Namespace) -> int:
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        sys.stderr.write("stoa-crystallize: ANTHROPIC_API_KEY not set; aborting.\n")
        return 2
    try:
        sources = _fetch_sources(args.question, args.top_k)
    except DaemonError as e:
        sys.stderr.write(f"stoa-crystallize: {e}\n")
        return 2
    if not sources:
        sys.stdout.write("stoa-crystallize: no wiki sources matched; aborting.\n")
        return 0
    body = _synthesize(args.model, api_key, args.question, sources)
    if not body:
        sys.stdout.write("stoa-crystallize: LLM returned empty synthesis.\n")
        return 0
    page_id = args.page_id or _default_page_id(args.question)
    frontmatter = _build_frontmatter(page_id, args.question, sources)
    return _emit_page(page_id, frontmatter, body, dry_run=args.dry_run)


def _build_frontmatter(
    page_id: str,
    question: str,
    sources: list[dict[str, Any]],
) -> dict[str, Any]:
    now = _dt.datetime.now(_dt.UTC).isoformat()
    return {
        "id": page_id,
        "title": question,
        "kind": "synthesis",
        "status": "active",
        "created": now,
        "updated": now,
        "question": question,
        "inputs": [s["page_id"] for s in sources],
    }


def _emit_page(
    page_id: str,
    frontmatter: dict[str, Any],
    body: str,
    *,
    dry_run: bool,
) -> int:
    if dry_run:
        sys.stdout.write(f"--- {page_id} ---\n{body}\n")
        return 0
    try:
        result = rpc("write_wiki", {"page_id": page_id, "frontmatter": frontmatter, "body": body})
    except DaemonError as e:
        sys.stderr.write(f"stoa-crystallize: write_wiki failed: {e}\n")
        return 2
    sys.stdout.write(f"stoa-crystallize: wrote {page_id} → {result.get('path', '?')}\n")
    return 0


def _fetch_sources(question: str, top_k: int) -> list[dict[str, Any]]:
    params: dict[str, Any] = {
        "query": question,
        "top_k": top_k,
        "filters": {"kind": "wiki"},
    }
    result = rpc("search", params)
    raw_hits = cast("list[Any]", result.get("hits") or [])
    out: list[dict[str, Any]] = []
    for h in raw_hits:
        if isinstance(h, dict):
            hit = cast("dict[str, Any]", h)
            out.append(
                {
                    "page_id": str(hit.get("doc_id", "")),
                    "snippet": str(hit.get("snippet", "")),
                    "path": str(hit.get("source_path", "")),
                },
            )
    return out


_SYSTEM_PROMPT = (
    "You are Stoa's crystallize worker. You produce honest, conservative "
    "synthesis pages by reading a set of wiki entries and answering a "
    "question across them. Output plain markdown only — no preamble, no "
    "code fences. Cite each source by its page_id (e.g., `ent-redis`)."
)


def _synthesize(
    model: str,
    api_key: str,
    question: str,
    sources: list[dict[str, Any]],
) -> str:
    try:
        import anthropic
    except ImportError:
        return ""
    client = anthropic.Anthropic(api_key=api_key)
    body_in = "\n\n".join(f"## {s['page_id']}\n{s['snippet']}" for s in sources)
    prompt = f"QUESTION:\n{question}\n\nSOURCES:\n{body_in}\n\nSynthesis:"
    msg = client.messages.create(
        model=model,
        max_tokens=2048,
        system=_SYSTEM_PROMPT,
        messages=[{"role": "user", "content": prompt}],
    )
    return _extract_text(msg)


def _extract_text(msg: Any) -> str:
    content = cast("list[Any]", getattr(msg, "content", []) or [])
    parts: list[str] = []
    for block in content:
        text = cast("str | None", getattr(block, "text", None))
        if isinstance(text, str):
            parts.append(text)
    return "\n".join(parts).strip()


def _default_page_id(question: str) -> str:
    slug = "-".join(token.lower() for token in question.split() if any(c.isalnum() for c in token))[
        :48
    ]
    return f"syn-{slug or 'unknown'}"


if __name__ == "__main__":
    sys.exit(main())
