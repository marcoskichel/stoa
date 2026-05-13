# Stoa

> The painted porch for AI memory.

Stoa is a **Rust hook surface + curated LLM wiki** layered over [MemPalace](https://github.com/MemPalace/mempalace). MemPalace stores every verbatim conversation; Stoa turns those into a curated knowledge wiki and injects wiki hits at every user prompt so the agent never has to remember to look anything up.

> **Pivot.** Stoa was rebuilt on 2026-05-13 around MemPalace as the recall backend. The from-scratch retrieval stack is gone. See [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md) for the why.

---

## Install

Requires Rust 1.95+ (via `rustup`) and `uv`.

```bash
# 1. Install MemPalace (the retrieval backend Stoa wraps)
uv tool install mempalace

# 2. Install Stoa
cargo install stoa-cli stoa-hooks stoa-inject-hooks --locked

# 3. Install Stoa's Python workers (daemon + harvest + crystallize)
uv tool install stoa-recalld
uv tool install stoa-harvest stoa-crystallize  # optional, requires ANTHROPIC_API_KEY
```

## Quickstart

```bash
# In any project directory
stoa init                       # scaffolds STOA.md, wiki/, .stoa/
stoa daemon start               # launches stoa-recalld in the background
stoa daemon status              # health probe

# Install hooks (prints a snippet; paste into ~/.claude/settings.json)
stoa hook install --platform claude-code --inject

# Write a wiki page
cat >/tmp/redis-fm.yaml <<EOF
id: ent-redis
title: Redis
status: active
kind: entity
type: library
created: 2026-05-13T00:00:00Z
updated: 2026-05-13T00:00:00Z
EOF
cat >/tmp/redis-body.md <<'EOF'
In-memory data store. Used for caching session tokens and rate limiting.
EOF
stoa write ent-redis --frontmatter /tmp/redis-fm.yaml --body /tmp/redis-body.md

# Search it
stoa query "redis caching" --top-k 5

# Inspect the injection audit log after a Claude Code session
stoa inject log --limit 5
```

## What you get

- **Per-prompt context injection.** `UserPromptSubmit` hook calls the daemon, fetches matching wiki hits, wraps them in `<stoa-memory>` with the MINJA-resistant defense, returns `additionalContext` so the agent sees the wiki at the top of every prompt.
- **Passive capture.** `SessionEnd` hook fires `mine` against the recall daemon — MemPalace indexes the transcript without the agent having to remember to save anything.
- **Curated wiki.** `stoa-harvest` periodically reads drawers, asks an LLM to identify durable entities + decisions, and writes them as `wiki/entities/*.md` / `wiki/concepts/*.md`. `stoa-crystallize` produces cross-page synthesis pages.
- **Local-first.** No cloud, no required API keys for retrieval (Anthropic only for harvest/crystallize). All data lives in your workspace.

## Architecture

Three layers:

1. **Stoa surface (Rust)** — `stoa-hook`, `stoa-inject-hook`, `stoa` CLI. Talks to the daemon over `$XDG_RUNTIME_DIR/stoa-recalld.sock`.
2. **`stoa-recalld` (Python)** — long-lived daemon that hosts MemPalace, owns the on-disk wiki, exposes 5 JSON-RPC methods.
3. **MemPalace** — the retrieval substrate. Hybrid BM25 + cosine, ChromaDB-backed, 96.6% R@5 on LongMemEval.

Wiki pages live as markdown on disk (`wiki/entities/`, `wiki/concepts/`, `wiki/synthesis/`) AND as drawers tagged `kind=wiki` inside the MemPalace palace. The on-disk file is canonical; the index is derived.

Full details: [ARCHITECTURE.md](./ARCHITECTURE.md). Positioning: [PRODUCT.md](./PRODUCT.md). Roadmap: [ROADMAP.md](./ROADMAP.md).

## Status

Pre-v0.1. Pivot landed; first tagged release is the next milestone.

## License

MIT — see [LICENSE](./LICENSE).
