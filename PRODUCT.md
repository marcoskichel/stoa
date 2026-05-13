# Stoa

> The painted porch for AI memory.

Stoa makes the knowledge an AI agent develops with you over time *load-bearing*: every user prompt gets relevant wiki context injected before the agent answers, and every session is mined for new entities + decisions that grow the wiki.

It does this by combining two open-source pieces:

1. **[MemPalace](https://github.com/MemPalace/mempalace)** — the fast, local, verbatim retrieval backend. 96.6% R@5 raw on LongMemEval. Stores every conversation, hybrid BM25 + cosine search, no API keys required.
2. **A curated LLM wiki** — markdown pages on disk (`wiki/entities/`, `wiki/concepts/`, `wiki/synthesis/`) shaped after Karpathy's [LLM Wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) pattern. Stoa harvests new entities + decisions out of MemPalace drawers and crystallizes synthesis pages from the wiki itself.

Stoa's contribution is **the layer between** — a Rust hook surface that injects wiki hits at every user prompt, MINJA-resistant envelope wrapping, and the Python workers that turn raw drawers into curated pages.

---

## The flow

```
You type a prompt
     │
     ▼
Claude Code fires UserPromptSubmit hook → stoa-inject-hook
     │
     ▼
stoa-inject-hook asks the daemon: "what wiki pages match this?"
     │   ~50-200 ms warm
     ▼
Top-K wiki hits with relevance scores
     │
     ▼
Wrapped in <stoa-memory> envelope (preamble + provenance + MINJA-safe)
     │
     ▼
Injected as additionalContext at the top of your prompt
     │
     ▼
Agent answers with the context already in front of it
```

The agent doesn't have to remember to look anything up. The wiki is *felt*.

---

## Audience

**Primary**: Claude Code power users who want a durable knowledge layer that survives across sessions, hooks into the existing agent loop, and doesn't require any third-party MCP plumbing.

**Secondary**: small engineering teams who want a shared brain — a workspace the team writes to once and reads from forever.

**Not the audience**: enterprises shopping for a Notion alternative. Stoa is dev-first, local-first, MIT, and built for agents.

---

## What's in the box

| Layer | What it does | Implemented as |
|---|---|---|
| **Retrieval backend** | Verbatim capture, hybrid BM25 + cosine, knowledge graph | [MemPalace](https://github.com/MemPalace/mempalace) (MIT) |
| **Recall daemon** | Hosts MemPalace, owns the disk wiki, JSON-RPC over Unix socket | `stoa-recalld` (Python) |
| **Hooks** | `SessionEnd` → mine; `UserPromptSubmit`/`SessionStart` → inject | `stoa-hook`, `stoa-inject-hook` (Rust) |
| **CLI** | Workspace + wiki + daemon orchestration | `stoa` (Rust) |
| **Harvest** | LLM-distills MemPalace drawers into wiki pages | `stoa-harvest` (Python) |
| **Crystallize** | LLM-synthesizes cross-page answers as `kind: synthesis` pages | `stoa-crystallize` (Python) |

The wiki on disk is canonical. The MemPalace palace is derived. Delete it, regenerate it.

---

## What makes this different

The market has split memory in two:

- **Wiki-side** (Karpathy's gist, lucasastorian/llmwiki) → curated pages, no recall layer. Search is grep.
- **Memory-side** (MemPalace, mem0, supermemory, zep, letta) → store + retrieve verbatim. Never compile, lint, or synthesize.

A handful of attempted integrations (Memoriki, wiki-recall) appeared in April 2026 and were abandoned within days. Memoriki was a 3-commit prompt template; wiki-recall fabricated its headline benchmark and stripped its memory layer in the final commit.

Stoa's bet is that **the wiki and the memory should be the same retrieval surface**. MemPalace stores drawers + Stoa's wiki pages in the same palace, tagged for separation. The agent's `UserPromptSubmit` injection pulls **wiki hits by default**, falls back to drawers when wiki coverage is thin. The harvest worker reads drawers, the crystallize worker reads wiki — same backend, same scoring, no second index to keep in sync.

---

## OSS core (MIT)

- `stoa init` — scaffold workspace
- `stoa daemon start|stop|status` — lifecycle the recall daemon
- `stoa hook install --platform claude-code` — print hook config snippet (user pastes; Stoa never mutates settings)
- `stoa write` / `stoa read` / `stoa query` — wiki I/O
- `stoa schema` / `stoa schema --check` — schema print + validate
- `stoa inject log` — audit-log tail
- `stoa-harvest run` / `stoa-crystallize run` — one-shot LLM workers (Anthropic by default, swappable)
- MINJA-resistant `<stoa-memory>` envelope with audit log on every injection
- `RecallBackend` trait — single MemPalace impl in v0.1, additional adapters welcome

## Paid layer (planned, not promised)

Built only after OSS adoption justifies the work. Obsidian's playbook: free local, paid sync.

- **Sync** — encrypted multi-device sync of `wiki/` + `.stoa/palace/`
- **Team** — shared brain across small engineering teams
- **Hosted** — managed instance for users who don't want to run it
- **Audit** — provenance log, decision tracking, citation export

The OSS core stays MIT and remains useful by itself. If the paid layer never ships, the core still works.

---

## What Stoa is *not*

- A MemPalace fork. Stoa depends on upstream MemPalace; we contribute issues back, not patches in-tree.
- A replacement for grep, IDE search, or your codebase. The wiki is for *decisions* and *durable entities*, not source code.
- A second-brain SaaS. v0.1 has no cloud component, no hosted service, no account.
- An MCP server. Stoa hooks into the platform's native agent hooks (`SessionStart`, `UserPromptSubmit`, `SessionEnd`) — agents don't need to remember to call a tool.

---

## Status

Pre-v0.1. Pivot landed on 2026-05-13; first tagged release is the next milestone. See [ROADMAP.md](./ROADMAP.md) for what blocks v0.1 and [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md) for the pivot rationale.
