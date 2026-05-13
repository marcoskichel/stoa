# Stoa

> The painted porch for AI memory.

Stoa is an open-core knowledge + memory system for AI agents. Every agent
session is captured to disk as plain markdown, distilled into a wiki of
entities and decisions, and surfaced back into the next session by
meaning — through the agent's existing hook surface.

The pattern that makes Stoa load-bearing rather than another retrieval
shim is the **three-layer split**:

- **Layer 1 — Wiki.** Markdown files under `wiki/`, `raw/`, and
  `sessions/`. Human-readable, Obsidian-compatible, git-trackable.
  This is the canonical store. If Stoa disappeared tomorrow, your files
  stay.
- **Layer 2 — Recall.** `.stoa/recall.db` and `.stoa/vectors/` hold the
  BM25 + embeddings + KG index. Everything here is derived from Layer 1
  and rebuildable with `stoa rebuild`. Nothing lives only here.
- **Layer 3 — CLI + hooks.** The `stoa` CLI and the `stoa-hook` /
  `stoa-inject-hook` binaries — the agent-facing surface.

```
 Agent SessionEnd hook
         │
         ▼
  .stoa/queue.db   ──►  Capture worker  ──►  sessions/ (redacted JSONL)
                              │
                              ▼
               Harvest worker  ──►  wiki/ (entities, concepts, synthesis)
                              │
                              ▼
            Recall index (BM25 + vectors + KG)
                              │
                              ▼
         SessionStart hook  ──►  top-K wiki pages injected into context
```

Two patterns are non-negotiable across the design:

1. **The Wiki / Recall split.** Layer 2 is always rebuildable from
   Layer 1. This is why the index is local-first and disposable.
2. **Hook → queue → worker.** Hooks complete in **<10 ms p95** — they
   insert one row into `.stoa/queue.db` and return. All heavy work
   (redaction, embedding, harvest, crystallize) runs in async workers
   draining the queue.

## Status

Pre-v0.1. The capture pipeline, recall, and SessionStart injection have
landed on `main`; v0.1 is the next tag and ships the public install +
benchmark numbers. Track the milestone plan in
[ROADMAP.md](https://github.com/marcoskichel/stoa/blob/main/ROADMAP.md).

## Next

- [Install](install.md) — get the binary on your machine.
- [Quickstart](quickstart.md) — 5 commands to a working workspace.
- [Wiki schema](schema.md) — what goes in `STOA.md`, how frontmatter is
  validated.
- [Capture pipeline](capture.md) — how sessions become files.
- [Recall](recall.md) — how `stoa query` finds things.
- [SessionStart injection](injection.md) — what Stoa puts in front of
  the agent and how MINJA-resistant wrapping protects it.
- [Troubleshooting](troubleshooting.md) — common failure modes.

See also: [ARCHITECTURE.md](https://github.com/marcoskichel/stoa/blob/main/ARCHITECTURE.md)
is the authoritative source of truth for the design.
[PRODUCT.md](https://github.com/marcoskichel/stoa/blob/main/PRODUCT.md)
covers positioning.
