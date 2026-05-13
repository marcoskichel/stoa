# Stoa

> The painted porch for AI memory.

Andrej Karpathy's [LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
sketched the right shape for an agent's long-term memory: markdown pages, curated by the model itself,
that compound across sessions. The gist was missing three things — a recall layer, an injection step,
and a capture path that doesn't depend on the agent remembering to write. Stoa is the working version:
hybrid recall (vector + BM25 + a small typed knowledge graph), SessionStart injection into Claude Code
today, Cursor and Codex next.

[![CI](https://github.com/marcoskichel/stoa/actions/workflows/rust.yml/badge.svg)](https://github.com/marcoskichel/stoa/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

→ Read [PRODUCT.md](./PRODUCT.md) for the full why.

---

## How it works

Three layers, each independently useful:

- **Wiki** (`wiki/`, `raw/`, `sessions/`) — plain markdown on disk. Human-readable, Obsidian-compatible,
  git-trackable. The canonical store; everything else is derived from it. If Stoa disappeared tomorrow,
  your files stay.
- **Recall** (`.stoa/recall.db`, `.stoa/vectors/`) — hybrid index over the wiki and session transcripts:
  vector embeddings + BM25 (SQLite FTS5) + a small typed knowledge graph. Behind a formal `RecallBackend`
  trait so the substrate is swappable. Fully rebuildable with `stoa rebuild`.
- **Capture + injection** — a deterministic agent-platform hook captures session transcripts into a SQLite
  queue in <10 ms. Async workers handle redaction, harvest, and crystallization off the agent's hot path.
  At session boot, Stoa injects the top-K relevant wiki pages into the agent's context automatically.

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

→ See [ARCHITECTURE.md](./ARCHITECTURE.md) for diagrams, invariants, and design rationale.

---

## Status

> **Pre-v0.1 — early development.**
>
> Repo skeleton and capture pipeline are merged. The walking skeleton — CLI + recall + SessionStart
> injection — is the v0.1 target.
>
> | What ships in v0.1 | What comes later |
> |---|---|
> | `stoa init`, `stoa query`, `stoa ingest`, `stoa note` | Harvest + crystallize workers (v0.2) |
> | Claude Code `Stop` hook capture + PII redaction | Cursor / Codex adapters (v0.3) |
> | `LocalChromaSqliteBackend` (vector + BM25 + KG) | MCP wrapper (v0.3) |
> | SessionStart injection with MINJA-resistant delimiters | UserPromptSubmit / PreCompact injection (v0.2) |
> | Reproducible benchmark suite (LongMemEval + 4 others) | Web UI (v0.4) |
> | Python sidecar for harvest/crystallize/embeddings | All-Rust sidecar replacement (v0.3) |
>
> No dates promised. Shipped honestly or not at all.

→ Full milestone plan: [ROADMAP.md](./ROADMAP.md) (MVP) · [ROADMAP-POST-MVP.md](./ROADMAP-POST-MVP.md) (v0.2 → v1.0)

---

## Install

**Pre-release (current):**

```bash
cargo install --git https://github.com/marcoskichel/stoa stoa-cli
```

**Stable (once v0.1 ships):**

```bash
cargo install stoa-cli
```

The Python sidecar (harvest, crystallize, embeddings) is an implementation detail — it bootstraps
automatically via `uv` on the first `stoa daemon` run. No manual sidecar management required.

**Platforms:** Linux x86\_64 / aarch64 · macOS x86\_64 / aarch64 · Windows x86\_64

---

## Quickstart

```bash
# Scaffold a Stoa workspace in your project directory
stoa init

# Register the Claude Code capture + injection hooks
stoa hook install --platform claude-code --inject session-start

# Start the background worker (capture → redact → queue)
stoa daemon &

# Use Claude Code normally. After a session ends, query what was captured:
stoa query "what did we decide about auth"

# Verify what was injected into your last session
stoa inject log
```

After a few sessions, Stoa accumulates a `wiki/` of entities and decisions your agent sessions have
touched. `stoa query` searches across wiki pages and session transcripts with hybrid recall (vector + BM25).
`stoa inject log` shows exactly which pages were prepended to the system prompt and why.

---

## What's in the OSS core (MIT)

- **`stoa init`** — scaffold workspace (`STOA.md`, `wiki/`, `raw/`, `sessions/`, `.stoa/`, `.gitignore`)
- **`stoa hook install`** — register capture and injection hooks for Claude Code (Cursor and Codex in v0.3)
- **`stoa daemon`** — run capture + harvest + scheduler workers
- **`stoa ingest`** — ingest URLs, PDFs, markdown, plain text into `raw/`
- **`stoa query`** — hybrid search across wiki + sessions (any agent, any shell)
- **`stoa inject log`** — inspect what was injected into recent sessions and why
- **`stoa harvest`** / **`stoa crystallize`** — manual triggers for distillation stages (automated in v0.2)
- **`stoa lint`** — wiki health check
- **`stoa note`** — add a structured observation to the active session (agent or human)
- **`stoa rebuild`** — regenerate all of `.stoa/` from `wiki/` + `sessions/` + `raw/`
- `LocalChromaSqliteBackend` as the default recall substrate; formal `RecallBackend` trait for community adapters
- Rule-based PII and secret redaction at capture and ingest; MINJA-resistant XML delimiters on every injection
- Always-flush on session exit — no `SAVE_INTERVAL` gate, no silent data loss
- Reproducible benchmark suite (LongMemEval, MemoryAgentBench, MEMTRACK, BEAM, AgentLeak) with published per-backend results
- Local-first — no required cloud, no required API keys

**Paid layer (planned, not promised):** sync, team shared brain, hosted instance, audit log.
The OSS core stays MIT and stays useful by itself.
→ See [PRODUCT.md §Paid layer](./PRODUCT.md#paid-layer-planned-not-promised)

---

## Benchmarks

No numbers yet — they ship with v0.1, pinned to the exact commit that produced them, across the
[LongMemEval, MemoryAgentBench, MEMTRACK, BEAM, and AgentLeak suites](./benchmarks/README.md).
Every `RecallBackend` adapter publishes against the same corpus; results that can't be reproduced
from a clean checkout don't land in the table.

---

## Repo layout

### Rust workspace

| Crate | Role |
|---|---|
| `stoa-core` | Schema, frontmatter, IDs |
| `stoa-cli` | `stoa` binary (clap) |
| `stoa-hooks` | `stoa-hook` binary; <10 ms cold-start budget |
| `stoa-queue` | SQLite-backed work queue |
| `stoa-capture` | Capture worker + PII redaction |
| `stoa-recall` | `RecallBackend` trait + reciprocal rank fusion |
| `stoa-recall/backends/local-chroma-sqlite` | Default v0.1 backend |
| `stoa-viz` | Visualization module + worker |
| `stoa-render-{mermaid,svg,tui}` | Render backends (resvg, ratatui+sixel) |
| `stoa-bench` | LongMemEval + benchmark runner |

### Python sidecar (`python/`, transitional — deleted at v0.3)

| Package | Role |
|---|---|
| `stoa-shared` | Shared queue client |
| `stoa-harvest` | Per-session entity extraction (`instructor` + `anthropic`) |
| `stoa-crystallize` | Nightly synthesis + invalidation pass |
| `stoa-embed` | Embedding worker (`sentence-transformers`) |

`benchmarks/spike-m0/` is excluded from the Cargo workspace (M0 validation spike, frozen).

---

## Contributing

`just ci` is the single local gate — runs fmt, clippy, tests, basedpyright, ruff, file length caps,
cargo-deny, and cargo-machete. All CI failures are real failures; `--no-verify` is never acceptable.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`.

→ Full setup and conventions: [CONTRIBUTING.md](./CONTRIBUTING.md)

---

## License

[MIT](./LICENSE) — Marcos Kichel and contributors.
