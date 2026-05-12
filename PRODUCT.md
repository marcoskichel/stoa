# Stoa

> The painted porch for AI memory.

Stoa is an open-core knowledge + memory system for AI agents. It implements Andrej Karpathy's LLM Wiki pattern (compounding markdown pages curated by the LLM) and pairs it with a hybrid recall layer (BM25 + embeddings + a small knowledge graph), exposed through an MCP server that drops into Claude Code, Cursor, Codex, and any MCP-aware client.

The name comes from the *Stoa Poikile* — the Painted Porch in the Athenian Agora where Zeno of Citium gathered his school around 300 BCE. Knowledge accreted there through paced conversation. Stoa-the-tool is the same idea for AI agents: a place where what gets thought once stays thought, and compounds.

## The problem

AI agents have session amnesia. Every conversation rediscovers what the previous one already knew. The market has split this problem in two and shipped neither half cleanly:

- **Wiki-side projects** (Karpathy's gist, lucasastorian/llmwiki, Astro-Han/karpathy-llm-wiki) compile knowledge into markdown but have no recall layer — search is grep.
- **Memory-side projects** (mempalace, mem0, supermemory, cognee, zep, letta, memmachine) store and retrieve well but never compile, synthesize, lint, or crystallize. Several launched with inflated benchmarks and missing features documented in their own issue trackers.

Two attempted integrations (Memoriki, wiki-recall) appeared in April 2026, both AI-vibe-coded in a single burst, both abandoned within days. Memoriki is a 3-commit prompt template. wiki-recall fabricated its headline benchmark numbers and stripped its memory layer in its final commit.

The gap is real. Nobody has shipped a working wiki + memory combo with honest benchmarks and a sustainable maintenance posture.

## Audience

**Primary**: Claude Code, Cursor, and Codex power users who already think in MCP and want a knowledge substrate that survives the session.

**Secondary**: small engineering teams who want a shared brain across agents — a workspace the team writes to once and reads from forever.

**Not the audience**: enterprises looking for a second-brain SaaS competitor to Notion. Stoa is dev-first, local-first, and built for agents.

## Architecture

Three layers, each addressable on its own:

1. **Wiki** — plain markdown on disk. Karpathy-compatible directory tree (`raw/`, `wiki/entities/`, `wiki/concepts/`, `wiki/synthesis/`, `index.md`, `log.md`). Obsidian-readable. Git-trackable. The LLM writes and lints these files directly through standard file tools.

2. **Recall** — hybrid index over the wiki and over raw session transcripts. BM25 + local embeddings (no API calls required) + a small knowledge graph for entity relationships. Stored locally in SQLite + a chosen vector backend (default LanceDB; ChromaDB available).

3. **MCP server** — exposes ingest, query, lint, and crystallize operations as tools any MCP client can call. One install, one config block, drops into Claude Code / Cursor / Codex.

Two background processes sit on top:

- **Lint** — deterministic auto-fixes (broken links, orphan pages, frontmatter validation) and heuristic reports (suspected contradictions, stale claims, missing entities). Runs on schedule or on-demand.
- **Crystallize** — periodically promotes high-signal episodes from the recall layer into structured wiki pages, closing the loop between episodic and semantic memory. Karpathy's gist describes this; rohitg00's LLM Wiki v2 elaborates it; nobody has shipped it.

## OSS core (MIT)

- `stoa init` — scaffold wiki + config in any directory
- `stoa ingest` — ingest URLs, PDFs, markdown, plain text
- `stoa query` — local hybrid search across wiki + sessions
- `stoa lint` — wiki health check
- `stoa crystallize` — promote recall content to wiki pages
- MCP server with tools matching the CLI
- Reproducible LongMemEval benchmark scripts published from day one
- Local-first: no required cloud, no API keys

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

4. **Local-first, MCP-native.** Runs entirely on the user's machine. Drops into Claude Code, Cursor, Codex via MCP with one config block. No mandatory cloud, no required API key.

5. **Markdown on disk.** The wiki is plain files. Users can grep them, edit them in Obsidian, version them in git, take them with them. Recall is an index over those files, not the canonical store.

## Business model

Open core. MIT-licensed core stays useful indefinitely. Paid layer (sync, team, hosted, audit) added once OSS adoption justifies the build. Reference precedents: Obsidian (free local, paid sync), Supabase, Plausible, Linear's early days.

Not building: pure SaaS competitor to Notion / mem.ai / Reflect — that lane is a graveyard. Not building: pure OSS with no monetization plan — burn-out path. The middle path is the only one with a track record in this category.

## Roadmap (rough)

- **v0.1 — walking skeleton.** CLI + markdown wiki + basic BM25 recall + MCP server. Reproducible LongMemEval benchmark. Public.
- **v0.2 — hybrid recall.** Embeddings + small KG. Lint operation with deterministic + heuristic checks.
- **v0.3 — crystallization.** Promote-to-wiki loop. Decay/staleness flags.
- **v0.4 — multi-agent.** Shared brain across agents. Conflict resolution.
- **v1.0 — production.** Hardening, audit log, encryption-at-rest. Begin paid layer evaluation.

No dates promised. Shipped honestly or not at all.

## Naming

Stoa (στοά) — a Greek covered walkway, typically a long colonnaded portico facing a public space. The most famous was the *Stoa Poikile* (the "Painted Porch") in the Athenian Agora, where Zeno of Citium taught around 300 BCE. His school took its name from the building: Stoicism. The Stoa was a place where knowledge gathered through paced conversation, in the open air, available to anyone who walked by.

The AI memory naming graveyard (mempalace, mem0, supermemory, cognee, zep, letta, memmachine, mycelium ×4, pith, smriti, crux, lore, tome, rune, etched, glean, sift, alembic, stratum, loci) made the short-natural-word space unworkable. Stoa survived because nothing else in the AI agent memory category claimed it.

## Status

Pre-v0.1. PRODUCT.md is the only artifact. Code, benchmarks, and MCP server to follow.
