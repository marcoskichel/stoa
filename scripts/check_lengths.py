#!/usr/bin/env python3
"""Enforce hard line caps that ruff cannot express today (May 2026).

- Function body <= 25 raw lines (including blanks/comments inside the body).
- File <= 400 raw lines.
"""

from __future__ import annotations

import ast
import sys
from pathlib import Path

MAX_FUNC_LINES = 25
MAX_FILE_LINES = 400


def check(path: Path) -> list[str]:
    src = path.read_text(encoding="utf-8")
    errors: list[str] = []
    n_lines = src.count("\n") + 1
    if n_lines > MAX_FILE_LINES:
        errors.append(f"{path}:1: file has {n_lines} lines (> {MAX_FILE_LINES})")
    tree = ast.parse(src, filename=str(path))
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if not node.body:
                continue
            start = node.body[0].lineno
            end = node.end_lineno or start
            length = end - start + 1
            if length > MAX_FUNC_LINES:
                errors.append(
                    f"{path}:{node.lineno}: function {node.name!r} body is "
                    f"{length} lines (> {MAX_FUNC_LINES})"
                )
    return errors


def main() -> int:
    paths = [Path(p) for p in sys.argv[1:]] or list(Path("python").rglob("*.py"))
    failures: list[str] = []
    for p in paths:
        if p.suffix == ".py" and ".venv" not in p.parts:
            failures.extend(check(p))
    for msg in failures:
        print(msg, file=sys.stderr)
    if not failures:
        print(f"OK: all .py files under cap (fn<={MAX_FUNC_LINES}, file<={MAX_FILE_LINES})")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
