"""`stoa-harvest` CLI entry point.

Pulls drawers from MemPalace via the stoa-recalld daemon, runs them
through a [`Distiller`][stoa_harvest.distill.Distiller], and writes the
resulting wiki pages back via the daemon's ``write_wiki`` RPC.

Usage:
    stoa-harvest run --query "..." [--top-k N] [--dry-run]
    stoa-harvest run --query "..." --model claude-opus-4-7
"""
# pyright: reportAny=false, reportUnknownMemberType=false, reportUnknownArgumentType=false, reportUnknownVariableType=false

from __future__ import annotations

import argparse
import json
import sys
from typing import Any, cast

from stoa_harvest.client import DaemonError, rpc
from stoa_harvest.distill import AnthropicDistiller, DrawerSnippet, PageDraft


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="stoa-harvest", description="Stoa LLM harvest worker.")
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="Pull recent drawers and produce wiki drafts.")
    _ = run.add_argument("--query", default="", help="Optional drawer-selection query")
    _ = run.add_argument("--top-k", type=int, default=20, help="Drawer batch size")
    _ = run.add_argument("--model", default="claude-opus-4-7", help="Anthropic model id")
    _ = run.add_argument("--dry-run", action="store_true", help="Print drafts; do not write")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    if args.command == "run":
        return _cmd_run(args)
    return 2


def _cmd_run(args: argparse.Namespace) -> int:
    try:
        drawers = _fetch_drawers(args.query, args.top_k)
    except DaemonError as e:
        sys.stderr.write(f"stoa-harvest: {e}\n")
        return 2
    if not drawers:
        sys.stdout.write("stoa-harvest: no drawers matched; nothing to harvest.\n")
        return 0
    distiller = AnthropicDistiller(model=args.model)
    if not distiller.available():
        sys.stderr.write(
            "stoa-harvest: ANTHROPIC_API_KEY not set or anthropic SDK missing. Skipping.\n",
        )
        return 0
    drafts = distiller.distill(drawers)
    if not drafts:
        sys.stdout.write("stoa-harvest: no drafts produced.\n")
        return 0
    sys.stdout.write(f"stoa-harvest: produced {len(drafts)} drafts.\n")
    return _emit_drafts(drafts, dry_run=args.dry_run)


def _emit_drafts(drafts: list[PageDraft], *, dry_run: bool) -> int:
    if dry_run:
        for d in drafts:
            sys.stdout.write(_render_draft(d))
        return 0
    for d in drafts:
        try:
            result = rpc(
                "write_wiki",
                {"page_id": d.page_id, "frontmatter": d.frontmatter, "body": d.body},
            )
        except DaemonError as e:
            sys.stderr.write(f"stoa-harvest: write_wiki {d.page_id} failed: {e}\n")
            continue
        sys.stdout.write(f"  wrote {d.page_id} → {result.get('path', '?')}\n")
    return 0


def _fetch_drawers(query: str, top_k: int) -> list[DrawerSnippet]:
    params: dict[str, Any] = {
        "query": query or "decision",
        "top_k": top_k,
        "filters": {},
    }
    result = rpc("search", params)
    raw_hits = cast("list[Any]", result.get("hits") or [])
    out: list[DrawerSnippet] = []
    for h in raw_hits:
        if not isinstance(h, dict):
            continue
        hit = cast("dict[str, Any]", h)
        if hit.get("metadata", {}).get("kind") == "wiki":
            continue
        out.append(
            DrawerSnippet(
                drawer_id=str(hit.get("doc_id", "")),
                text=str(hit.get("snippet", "")),
                source_file=str(hit.get("source_path", "")),
            ),
        )
    return out


def _render_draft(d: PageDraft) -> str:
    return f"--- {d.page_id} ---\n{json.dumps(d.frontmatter, indent=2)}\n\n{d.body}\n\n"


if __name__ == "__main__":
    sys.exit(main())
