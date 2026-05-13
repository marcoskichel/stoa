# ADR-0001: Wrap MemPalace instead of building our own retrieval substrate

- **Status**: Accepted
- **Date**: 2026-05-13
- **Decider**: Marcos Kichel (project lead)

## Context

Stoa was started in early 2026 to ship a working **LLM wiki + memory** combo with honest benchmarks, addressing the gap between the wiki-side projects (Karpathy's gist, llmwiki) and the memory-side projects (mem0, supermemory, zep, letta, MemPalace). The original plan was to build the retrieval substrate from scratch — own ChromaDB integration, own SQLite FTS5 BM25, own small typed KG — behind a swappable `RecallBackend` trait, with MemPalace and others as future community-maintained adapters.

By the end of M5, the from-scratch substrate had landed: `stoa-capture` (queue-based hook, regex PII redaction, capture worker), `stoa-recall` with `LocalChromaSqliteBackend`, `stoa-bench` scaffolded for LongMemEval + MemBench + LoCoMo + MEMTRACK + AgentLeak. The next milestone (M6) was the release on-ramp.

Two things became clear at the M5 → M6 boundary:

1. **MemPalace shipped what Stoa was building.** MemPalace publishes verbatim drawer storage, hybrid BM25 + cosine search, a 29-tool MCP server, Claude Code auto-save hooks, AND benchmarks higher than Stoa's targets — 96.6% R@5 raw on LongMemEval, 98.4% held-out hybrid, 92.9% on ConvoMem. The retrieval problem Stoa was solving had been solved well by a parallel project with the same MIT license and the same local-first stance.
2. **Stoa's actual value was upstream of retrieval.** What made Stoa interesting wasn't ChromaDB integration; it was (a) the curated LLM wiki on disk in Karpathy's pattern, (b) the per-prompt injection with MINJA-resistant wrapping, and (c) the Rust <10 ms hook surface. None of those depend on owning the retrieval substrate.

## Decision

**Stoa wraps MemPalace as the v0.1 retrieval backend. The from-scratch substrate is deleted, not deprecated.**

Concretely:

- Delete the Rust crates that implemented or supported the from-scratch substrate: `stoa-queue`, `stoa-capture`, `stoa-bench`, `stoa-viz`, all `stoa-render-*`, `stoa-recall/backends/local-chroma-sqlite`.
- Delete the Python sidecar packages that no longer earn their keep: `stoa-shared` (queue client), `stoa-embed` (fastembed wrapper), the Python `stoa-recall` sidecar, `stoa-bench-judge`.
- Keep `stoa-core` (wiki schema), `stoa-cli` (thin orchestrator), `stoa-hooks` + `stoa-inject-hooks` (the Rust surface), `stoa-recall` (now just the `RecallBackend` trait + `MempalaceBackend` adapter), `stoa-doclint`.
- Keep `stoa-harvest` + `stoa-crystallize` as the LLM workers — but rewire them to talk to MemPalace via the new daemon, not to the deleted queue.
- Add `stoa-recalld`: a long-lived Python daemon that hosts MemPalace in-process and exposes a 5-method JSON-RPC surface over a Unix domain socket. This is the seam between Rust (hooks + CLI) and Python (MemPalace + LLM workers).

The `RecallBackend` trait survives even though only MemPalace implements it. The cost of keeping the abstraction is ~50 lines of Rust; the optionality is real (if a better backend ships next year, the daemon switches sides without touching the hooks or CLI).

## Consequences

**Positive:**

- v0.1 ships in ~1 week instead of ~1 quarter. The benchmark battle is over.
- Stoa cites MemPalace's published recall numbers rather than running its own; the comparison is honest and reproducible.
- The Rust hook surface stays Stoa's load-bearing contribution (<10 ms `stoa-hook`, <500 ms `stoa-inject-hook` warm) — MemPalace's hooks are Python and cold-start too slow for `UserPromptSubmit` injection on every prompt.
- The MINJA-resistant injection envelope + audit log become unambiguously Stoa's value — MemPalace explicitly does not provide this.

**Negative:**

- v0.1 depends on a third-party Python project (MemPalace) on every machine where Stoa is installed. The `cargo install stoa-cli` path no longer self-bootstraps — users need `uv tool install mempalace` first.
- The pluggable backend story is now hypothetical until a second backend appears. Until then, "swappable retrieval" is marketing.
- We inherit MemPalace's bug surface. Stoa users will hit MemPalace issues that are out of our control. Mitigation: pin a known-good version range (`mempalace>=3.3.5,<4`).
- Pre-pivot crates published to crates.io at 0.1.0 are obsolete. They get yanked; a clean 0.1.0 republish ships the pivoted code.

**Neutral:**

- The "wiki on disk is canonical" property survives intact. MemPalace stores drawers in ChromaDB, but Stoa writes wiki pages to `wiki/*.md` first AND mirrors them into MemPalace tagged `kind=wiki`. The markdown files are the source of truth; the palace is derived.
- The Karpathy LLM Wiki pattern (entities / concepts / synthesis on disk) is unchanged. So is the schema in `STOA.md`.

## Alternatives considered

1. **Keep the from-scratch substrate, ship around it.** Rejected — duplicates MemPalace's work; we'd be benchmark-chasing a project that already lapped us.
2. **Drop the `RecallBackend` trait entirely; hard-code MemPalace.** Rejected — 50 LOC of optionality for a real future seam is worth keeping.
3. **Make Stoa an MCP wrapper over MemPalace.** Rejected — MemPalace already ships a 29-tool MCP server. A Stoa MCP server would be redundant. Stoa's bet is on **passive injection** through `UserPromptSubmit`, not on an MCP tool the agent has to choose to call.
4. **Stay on MemPalace and drop the Rust hooks.** Rejected — MemPalace's Python hooks have cold-start costs that make per-prompt injection painful. The Rust hook surface is a real differentiator.

## Implementation

Single PR ("M-Pivot") deletes the obsolete code, ships the daemon + new RPC client, rewires the hooks + CLI + workers, and rewrites the docs. v0.1 release is the immediate next milestone — see [ROADMAP.md](https://github.com/marcoskichel/stoa/blob/main/ROADMAP.md) §M-v0.1.
