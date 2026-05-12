# Stoa Python sidecar workspace

Transitional uv workspace for v0.1–v0.2. Hosts:

- `stoa-harvest` — instructor + anthropic
- `stoa-crystallize` — instructor + anthropic
- `stoa-embed` — sentence-transformers
- `stoa-shared` — shared queue client

Deleted at v0.3 when the pipeline becomes all-Rust per [ARCHITECTURE.md §16.6](../ARCHITECTURE.md).

## Local dev

```bash
uv sync --all-groups
uv run pytest
uv run ruff check .
uv run basedpyright
```

Or use the workspace `Justfile` from the repo root: `just ci-python`.
