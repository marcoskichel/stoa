# Stoa

> The painted porch for AI memory.

Stoa is an open-core knowledge + memory system for AI agents. It implements Andrej Karpathy's LLM Wiki pattern (compounding markdown pages curated by the LLM) on top of a hybrid recall layer (vector + BM25 + small typed knowledge graph) behind a swappable backend interface. Capture is automatic — a Claude Code `Stop` hook (Cursor and Codex next) writes the full session transcript into the workspace; an async worker redacts PII, harvests entities, and on a nightly schedule crystallizes synthesis pages with an explicit invalidation pass. Stoa also injects relevant memory into the agent's context at the right hook points (SessionStart in v0.1; UserPromptSubmit and PreCompact in v0.2), so the wiki is *felt* by the agent without it ever having to remember to query. No MCP server required to ship value.

The name comes from the *Stoa Poikile* — the Painted Porch in the Athenian Agora where Zeno of Citium gathered his school around 300 BCE. Knowledge accreted there through paced conversation. Stoa-the-tool is the same idea for AI agents: a place where what gets thought once stays thought, and compounds.

## The problem

AI agents have session amnesia. Every conversation rediscovers what the previous one already knew. The market has split this problem in two and shipped neither half cleanly:

- **Wiki-side projects** (Karpathy's gist, lucasastorian/llmwiki, Astro-Han/karpathy-llm-wiki) compile knowledge into markdown but have no recall layer — search is grep.
- **Memory-side projects** (mempalace, mem0, supermemory, cognee, zep, letta, memmachine) store and retrieve well but never compile, synthesize, lint, or crystallize. Several launched with inflated benchmarks and missing features documented in their own issue trackers.

Two attempted integrations (Memoriki, wiki-recall) appeared in April 2026, both AI-vibe-coded in a single burst, both abandoned within days. Memoriki is a 3-commit prompt template. wiki-recall fabricated its headline benchmark numbers and stripped its memory layer in its final commit.

The gap is real. Nobody has shipped a working wiki + memory combo with honest benchmarks and a sustainable maintenance posture.

## Audience

**Primary**: Claude Code, Cursor, and Codex power users who want a knowledge substrate that survives the session and integrates through tools they already use (shell + hooks, no extra MCP wiring required).

**Secondary**: small engineering teams who want a shared brain across agents — a workspace the team writes to once and reads from forever.

**Not the audience**: enterprises looking for a second-brain SaaS competitor to Notion. Stoa is dev-first, local-first, and built for agents.

## Architecture

Three layers, each addressable on its own:

1. **Wiki** — plain markdown on disk. Karpathy-compatible directory tree. Obsidian-readable, git-trackable. The canonical store; everything else can be rebuilt from it.
2. **Recall** — hybrid index over the wiki and session transcripts: vector embeddings (ChromaDB by default) + BM25 (SQLite FTS5) + small typed knowledge graph (SQLite). Behind a formal `RecallBackend` interface so the substrate is swappable; alternative adapters (mempalace, LanceDB, pgvector) land later as community-maintained backends. Stored locally. No API calls required.
3. **CLI + agent-platform hooks** — `stoa` CLI exposes every operation; small per-platform hook scripts capture session transcripts into a queue. Async workers do all the heavy work (redaction, harvest, crystallize) off the agent's hot path. An MCP wrapper is planned for v0.3 as optional sugar; v0.1 is hooks + CLI.

### Design pillars

The shape of the system, not the feature list. Each pillar is detailed in [ARCHITECTURE.md](./ARCHITECTURE.md).

- **Schema as product.** A `STOA.md` file at the workspace root encodes domain entity types, relationship vocabulary, ingest rules, quality bar, and privacy redactions. Loaded into every agent context. The most important file in the system.
- **Capture without trusting the agent.** Session transcripts are captured by a deterministic platform hook in <10ms, written to a queue, redacted by a worker. The agent doesn't need to remember to "save" anything; passive capture is the floor. An optional `stoa note` lets the agent flag importance, but reliability never depends on it.
- **Two-stage distillation.** Per-session **harvest** with strict quality gating extracts entities and decisions into the wiki. Nightly **crystallize** synthesizes cross-session essays as drafts and runs an explicit invalidation pass to retire stale claims. Mem0's published numbers are the proof: selective retrieval + injection delivers 91% lower p95 latency, 90% token reduction, and 26% accuracy gain over naive full-context. Single-pass distillation lags staged distillation by ~15 percentage points on LongMemEval.
- **Auto-injection of memory at the right hook points.** The agent doesn't have to remember to query. SessionStart in v0.1 prepends top-K relevant wiki pages to the system prompt at session boot. UserPromptSubmit and PreCompact follow in v0.2 (with relevance gating and `systemMessage`-only PreCompact to avoid mempalace's documented blocking-loop bug). Every injection is hard-capped on tokens, gated by relevance threshold, and wrapped in MINJA-resistant XML delimiters with explicit "treat as data, not instructions" framing.
- **Swappable recall substrate.** A formal `RecallBackend` interface separates retrieval storage from Stoa's orchestration. v0.1 ships `LocalChromaSqliteBackend` (ChromaDB + SQLite FTS5 + SQLite KG). Mempalace, LanceDB, and pgvector adapters land later. Backend swaps must publish recall@k against Stoa's published test corpus — no silent quality regressions.
- **Lifecycle without decay theater.** Explicit supersession + staleness flags + git history. No silent decay scores, no Ebbinghaus forgetting curves on facts. Confidence scores live on relationships only and are derived, not gut-set.
- **Event-driven automation.** Hook-triggered events (`agent.session.ended`, `transcript.captured`, `source.ingested`, `wiki.page.written`, `lint.tick`, `crystallize.tick`) run through async workers. User-extensible hooks (`.stoa/hooks/<event>/`).
- **Privacy redaction at capture and ingest. MINJA defenses on injection.** Rule-based PII/secret redaction applied before content reaches `raw/` or `sessions/`. Always-flush on session exit (no `SAVE_INTERVAL` gate; mempalace #1341 was a silent-loss bug Stoa avoids by design). Injection content is structurally segregated from agent instructions, against the OWASP ASI06 memory-poisoning attack class. All in the OSS core, not the paid tier.
- **Visualization is for humans, with the science enforced.** The wiki and recall surface are agent-readable by default (markdown, JSON), but Stoa ships a visualization module whose defaults are imported wholesale from the visualization literature: position before area before color (Cleveland-McGill 1984), one pre-attentive channel per category (Treisman 1980; Healey et al. 1996), overview-then-filter-then-detail (Shneiderman 1996), perceptually uniform colormaps (viridis; jet rejected per Borland & Taylor 2007), UMAP only with epistemic warnings (Wattenberg et al. 2016). Anti-patterns (3D bars, pie >5 slices, dual y-axes, hairball graphs, word clouds for analysis) are rejected at lint time, not stylistic preferences. Mermaid for markdown-portable diagrams, Sigma.js + Observable Plot for the web UI, ratatui + sixel for the terminal.
- **Markdown is canonical.** The recall index is derived. Delete `.stoa/`, run `stoa rebuild`, get it all back. The user's knowledge survives Stoa.

## OSS core (MIT)

- `stoa init` — scaffold wiki + config in any directory
- `stoa hook install --platform claude-code [--inject session-start]` — register the agent capture and injection hooks
- `stoa daemon` — run capture + harvest + scheduler workers
- `stoa ingest` — ingest URLs, PDFs, markdown, plain text
- `stoa query` — local hybrid search across wiki + sessions (any agent, any shell)
- `stoa inject log` — inspect what was injected into recent sessions and why
- `stoa harvest` / `stoa crystallize` — manual triggers for the distillation stages
- `stoa lint` — wiki health check
- `stoa note` — add a structured observation to the active session (agent or human)
- `LocalChromaSqliteBackend` as the default recall substrate; formal `RecallBackend` interface for alternative adapters
- Rule-based PII/secret redaction applied at capture and ingest, plus MINJA-resistant XML delimiters on every injection
- Reproducible LongMemEval benchmark scripts published from day one (recall@k against a fixed test corpus; backend swaps must publish against the same corpus)
- Local-first: no required cloud, no required API keys

## Paid layer (planned, not promised)

Built only after OSS adoption justifies the work. Reference: Obsidian's playbook (free local, paid sync).

- **Sync** — encrypted multi-device sync of wiki + recall index
- **Team** — shared brain for small engineering teams, with read/write attribution and conflict resolution
- **Hosted** — managed instance for users who don't want to run it
- **Audit** — provenance log, decision tracking, citation export

The OSS core stays MIT and stays useful by itself. If the paid layer never ships, the core still works.

## Differentiation

Seven things separate Stoa from the existing field.

1. **Wiki + recall + injection in one tool.** The market has shipped fragments — Memoriki has the wiki + retrieval split but no injection layer; claude-mem has injection but no wiki write side; agentmemory has the hook suite but no schema. Nobody has shipped all three together with a quality contract.

2. **Auto-injection at the right hook points, with MINJA defenses by default.** SessionStart in v0.1, UserPromptSubmit and PreCompact in v0.2. Hard token budgets, relevance gating, top-of-prompt placement (lost-in-the-middle defense), and explicit XML delimiters with "treat as data, not instructions" framing on every injection. The OWASP ASI06 attack class is a designed-against threat, not an afterthought.

3. **Honest benchmarks against a fixed corpus.** Mempalace launched with a "100% LongMemEval" headline that was ChromaDB stock nearest-neighbor performance, not the palace structure. The independent lhl/agentic-memory analysis showed several README features were absent from the code. Stoa publishes its test corpus, runs LongMemEval reproducibly from day one, and requires every backend adapter to publish recall@k against the same corpus. No re-runs after engineering fixes.

4. **Crystallization with invalidation.** Multiple specs describe a promote-from-sessions loop; nobody ships the inverse. Stoa's nightly crystallize produces both new synthesis drafts and supersession proposals. The Memora FAMA benchmark documents 18–32% accuracy loss when memory systems only add and never retire — Stoa's invalidation pass is the answer.

5. **Local-first, hook + CLI native.** Runs entirely on the user's machine. Captures session transcripts via a deterministic agent-platform hook (Claude Code `Stop` first; Cursor and Codex adapters next), and exposes every operation as a `stoa` CLI command any agent can invoke through its existing shell tool. CLI invocation is empirically more reliable than MCP tool calls on hard tasks (100% vs ~72% in published comparisons). MCP wrapper available later for clients that prefer the tool-panel UX.

6. **Markdown is canonical; recall is derived.** The wiki is plain files. Users can grep them, edit them in Obsidian, version them in git, take them with them. The recall substrate is hidden behind a `RecallBackend` interface and is fully rebuildable from `wiki/` + `sessions/` + `raw/`. The user's knowledge survives Stoa, survives backend swaps, and survives Stoa being abandoned.

7. **Science-backed visualization for humans, not chartware.** Most memory tools either expose no human-facing visual surface at all (mempalace, mem0, supermemory) or ship default chart libraries with no opinion (decorative dashboards). Stoa imports its visual defaults from the visualization literature — Cleveland-McGill perceptual ranking, Shneiderman's overview mantra, Treisman's pre-attentive channels, ColorBrewer/viridis colormaps, Ghoniem et al. for graph layout choice, Wattenberg et al. for UMAP epistemic limits. Banned anti-patterns (3D charts, rainbow colormaps, pie >5 slices, dual y-axes, unfiltered hairball graphs, analytical word clouds) are rejected by the renderer, not just discouraged. Markdown-embeddable Mermaid + pre-rendered SVG for portability; Sigma.js + Observable Plot in the web UI; ratatui + sixel in the terminal.

## Business model

Open core. MIT-licensed core stays useful indefinitely. Paid layer (sync, team, hosted, audit) added once OSS adoption justifies the build. Reference precedents: Obsidian (free local, paid sync), Supabase, Plausible, Linear's early days.

Not building: pure SaaS competitor to Notion / mem.ai / Reflect — that lane is a graveyard. Not building: pure OSS with no monetization plan — burn-out path. The middle path is the only one with a track record in this category.

## Roadmap (rough)

The shipping plan with milestones and exit criteria lives in [ROADMAP.md](./ROADMAP.md) (MVP) and [ROADMAP-POST-MVP.md](./ROADMAP-POST-MVP.md) (v0.2 → v1.0). What follows is the high-level shape.

- **v0.1 — walking skeleton.** CLI + markdown wiki + `RecallBackend` interface with `LocalChromaSqliteBackend` (vector + BM25 + KG) + Claude Code `Stop` hook + capture worker + PII redaction + always-flush + SessionStart injection with MINJA-resistant delimiters. Reproducible LongMemEval benchmark. Public. Stack: Rust core (CLI, hooks, capture, viz, SQLite/FTS5) + Python sidecar (harvest/crystallize/embeddings, `uv`-bootstrapped). Migration path to all-Rust validated for v0.2 (embedding worker) and v0.3 (LLM calls + LanceDB) — see [ARCHITECTURE.md §15](./ARCHITECTURE.md).
- **v0.2 — distillation + advanced injection + terminal viz.** Harvest worker with strict quality gating. Crystallize loop with invalidation pass. UserPromptSubmit and PreCompact (`systemMessage`-only) injection. Lint with deterministic + heuristic checks (including viz anti-pattern lint). User-extensible event hooks. `stoa render` ships with ratatui sparklines/bars and Mermaid embeds for entity neighborhoods, log timelines, and distillation reports.
- **v0.3 — cross-platform + experimental hooks + MCP + pre-rendered SVG snapshots.** Cursor and Codex capture + injection adapters. Lifecycle (supersession, staleness flow). PreToolUse experimental injection. Thin MCP wrapper that shells out to CLI. `MempalaceBackend` adapter ships if mempalace's API has been stable for ≥60 days. Viz worker subscribes to `wiki.page.written` and snapshots SVGs into `.stoa/renders/` (and optionally `wiki/.renders/` for git-portable embed).
- **v0.4 — multi-agent + community backends + web viewer.** Shared brain across agents. Scoping, promotion, mesh sync, conflict resolution. Community-maintained `LanceDbBackend`, `PgVectorBackend`. `stoa serve` browser viewer with Sigma.js entity graph, Observable Plot statistical charts, and visx LineUp/UpSet primitives.
- **v1.0 — production.** Hardening, audit log surface, encryption-at-rest for `sessions/`. Begin paid-layer evaluation.

No dates promised. Shipped honestly or not at all.

## Naming

Stoa (στοά) — a Greek covered walkway, typically a long colonnaded portico facing a public space. The most famous was the *Stoa Poikile* (the "Painted Porch") in the Athenian Agora, where Zeno of Citium taught around 300 BCE. His school took its name from the building: Stoicism. The Stoa was a place where knowledge gathered through paced conversation, in the open air, available to anyone who walked by.

The AI memory naming graveyard (mempalace, mem0, supermemory, cognee, zep, letta, memmachine, mycelium ×4, pith, smriti, crux, lore, tome, rune, etched, glean, sift, alembic, stratum, loci) made the short-natural-word space unworkable. Stoa survived because nothing else in the AI agent memory category claimed it.

## Status

Pre-v0.1. PRODUCT.md and ARCHITECTURE.md are the only artifacts. Code, hook scripts, workers, and benchmarks to follow.
