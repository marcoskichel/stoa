# Stoa

> The painted porch for AI memory.

Stoa is an open-core knowledge + memory system for AI agents. It implements Andrej Karpathy's LLM Wiki pattern (compounding markdown pages curated by the LLM) and pairs it with a hybrid recall layer (BM25 + embeddings + a small knowledge graph). Capture is automatic — a Claude Code `Stop` hook (Cursor and Codex adapters next) writes the full session transcript into the workspace; an async worker redacts PII, harvests entities, and on a nightly schedule crystallizes synthesis pages. Agents query through `stoa query` via their existing shell tool. No MCP server required to ship value.

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
2. **Recall** — hybrid index over the wiki and session transcripts: BM25 + local embeddings + a small typed knowledge graph. Stored locally. No API calls required.
3. **CLI + agent-platform hooks** — `stoa` CLI exposes every operation; small per-platform hook scripts capture session transcripts into a queue. Async workers do all the heavy work (redaction, harvest, crystallize) off the agent's hot path. An MCP wrapper is planned for v0.3 as optional sugar; v0.1 is hooks + CLI.

### Design pillars

The shape of the system, not the feature list. Each pillar is detailed in [ARCHITECTURE.md](./ARCHITECTURE.md).

- **Schema as product.** A `STOA.md` file at the workspace root encodes domain entity types, relationship vocabulary, ingest rules, quality bar, and privacy redactions. Loaded into every agent context. The most important file in the system.
- **Capture without trusting the agent.** Session transcripts are captured by a deterministic platform hook in <10ms, written to a queue, redacted by a worker. The agent doesn't need to remember to "save" anything; passive capture is the floor. An optional `stoa note` lets the agent flag importance, but reliability never depends on it.
- **Two-stage distillation.** Per-session **harvest** with strict quality gating extracts entities and decisions into the wiki. Nightly **crystallize** synthesizes cross-session essays as drafts and runs an explicit invalidation pass to retire stale claims. Single-pass distillation lags this approach by ~15 percentage points on the leading benchmarks.
- **Lifecycle without decay theater.** Explicit supersession + staleness flags + git history. No silent decay scores, no Ebbinghaus forgetting curves on facts. Confidence scores live on relationships only and are derived, not gut-set.
- **Event-driven automation.** Hook-triggered events (`agent.session.ended`, `transcript.captured`, `source.ingested`, `wiki.page.written`, `lint.tick`, `crystallize.tick`) run through async workers. User-extensible hooks (`.stoa/hooks/<event>/`).
- **Privacy redaction at capture and ingest.** Rule-based PII/secret redaction applied before content reaches `raw/` or `sessions/`. In the OSS core, not the paid tier.
- **Markdown is canonical.** The recall index is derived. Delete `.stoa/`, run `stoa rebuild`, get it all back. The user's knowledge survives Stoa.

## OSS core (MIT)

- `stoa init` — scaffold wiki + config in any directory
- `stoa hook install --platform claude-code` — register the agent capture hook
- `stoa daemon` — run capture + harvest + scheduler workers
- `stoa ingest` — ingest URLs, PDFs, markdown, plain text
- `stoa query` — local hybrid search across wiki + sessions
- `stoa harvest` / `stoa crystallize` — manual triggers for the distillation stages
- `stoa lint` — wiki health check
- `stoa note` — add a structured observation to the active session (agent or human)
- Rule-based PII/secret redaction applied at capture and ingest
- Reproducible LongMemEval benchmark scripts published from day one
- Local-first: no required cloud, no required API keys

## Paid layer (planned, not promised)

Built only after OSS adoption justifies the work. Reference: Obsidian's playbook (free local, paid sync).

- **Sync** — encrypted multi-device sync of wiki + recall index
- **Team** — shared brain for small engineering teams, with read/write attribution and conflict resolution
- **Hosted** — managed instance for users who don't want to run it
- **Audit** — provenance log, decision tracking, citation export

The OSS core stays MIT and stays useful by itself. If the paid layer never ships, the core still works.

## Differentiation

Five things separate Stoa from the existing field.

1. **Wiki + recall in one tool.** Nobody else shipped this working. The wiki-only projects don't recall; the recall-only projects don't compile.

2. **Honest benchmarks.** Mempalace launched with a "100% LongMemEval" headline that was overfitted to the failing test cases, then republished as 96.6% after community pushback. Several README features (notably contradiction detection) are documented as missing in their own issues. Stoa publishes the benchmark scripts, the test sets, and the actual numbers — once. No re-runs after engineering fixes.

3. **Crystallization loop.** Episodic recall promoted to semantic wiki pages on a schedule. The feature multiple specs describe and nobody ships.

4. **Local-first, hook + CLI native.** Runs entirely on the user's machine. Captures session transcripts via a deterministic agent-platform hook (Claude Code `Stop` first; Cursor and Codex adapters next), and exposes every operation as a `stoa` CLI command any agent can invoke through its existing shell tool. CLI invocation is empirically more reliable than MCP tool calls on hard tasks (100% vs ~72% in published comparisons). MCP wrapper available later for clients that prefer the tool-panel UX.

5. **Markdown on disk.** The wiki is plain files. Users can grep them, edit them in Obsidian, version them in git, take them with them. Recall is an index over those files, not the canonical store.

## Business model

Open core. MIT-licensed core stays useful indefinitely. Paid layer (sync, team, hosted, audit) added once OSS adoption justifies the build. Reference precedents: Obsidian (free local, paid sync), Supabase, Plausible, Linear's early days.

Not building: pure SaaS competitor to Notion / mem.ai / Reflect — that lane is a graveyard. Not building: pure OSS with no monetization plan — burn-out path. The middle path is the only one with a track record in this category.

## Roadmap (rough)

- **v0.1 — walking skeleton.** CLI + markdown wiki + BM25 recall + Claude Code `Stop` hook + capture worker + PII redaction. Reproducible LongMemEval benchmark. Public.
- **v0.2 — distillation + hybrid recall.** Embeddings + small typed KG. Harvest worker with quality gating. Lint with deterministic + heuristic checks. User-extensible event hooks.
- **v0.3 — crystallize + lifecycle + cross-platform.** Crystallize loop with invalidation pass. Supersession + staleness flow. Cursor and Codex hook adapters. Optional MCP wrapper.
- **v0.4 — multi-agent.** Shared brain across agents. Scoping, promotion, mesh sync, conflict resolution.
- **v1.0 — production.** Hardening, audit log surface, encryption-at-rest for `sessions/`. Begin paid-layer evaluation.

No dates promised. Shipped honestly or not at all.

## Naming

Stoa (στοά) — a Greek covered walkway, typically a long colonnaded portico facing a public space. The most famous was the *Stoa Poikile* (the "Painted Porch") in the Athenian Agora, where Zeno of Citium taught around 300 BCE. His school took its name from the building: Stoicism. The Stoa was a place where knowledge gathered through paced conversation, in the open air, available to anyone who walked by.

The AI memory naming graveyard (mempalace, mem0, supermemory, cognee, zep, letta, memmachine, mycelium ×4, pith, smriti, crux, lore, tome, rune, etched, glean, sift, alembic, stratum, loci) made the short-natural-word space unworkable. Stoa survived because nothing else in the AI agent memory category claimed it.

## Status

Pre-v0.1. PRODUCT.md and ARCHITECTURE.md are the only artifacts. Code, hook scripts, workers, and benchmarks to follow.
