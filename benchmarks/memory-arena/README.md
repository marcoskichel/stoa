# MemoryArena

**Status**: Post-MVP. Gated on M5 (injection live) + budget for quarterly $500–2K runs.

## Source

[MemoryArena: Benchmarking Agent Memory in Interdependent Multi-Session Agentic Tasks](https://arxiv.org/abs/2602.16313) — He et al., Stanford / UCSD / UIUC / Princeton; Feb 2026.

## What it measures

766 tasks across web navigation, preference-constrained planning, progressive information search, sequential formal reasoning. Multi-session loops where earlier task outcomes are prerequisites for later ones. Average 57 action steps per task; traces exceed 40K tokens.

**Tests whether experience distilled into memory actually transfers to downstream task completion** — not just whether the right text can be retrieved.

## Why for Stoa

Memory benchmarks split into two kinds:
- Retrieval — can you find the text again (LongMemEval, BEAM, MemoryAgentBench).
- Agentic transfer — does the harvested wiki actually make the agent perform better on subsequent tasks.

MemoryArena is the canonical agentic-transfer benchmark. The paper's key finding — **near-saturated LoCoMo performance does not transfer to MemoryArena** — is the load-bearing reason Stoa needs both kinds in the suite.

## Cost

- 766 tasks × ~57 action steps × ~10 LLM calls per step.
- Roughly $500–2,000 per full run depending on backbone choice.
- No GPU.
- License: not stated explicitly; project page public.

Cadence: quarterly or release-gate, not monthly.

## Gameability notes

- No contamination analysis.
- No established leaderboard (too new).
- Task domains (web shopping, travel planning) don't perfectly mirror coding workflows. The formal reasoning + progressive search sub-tasks are domain-agnostic and the more honest signal for Stoa.
- No memory vendor has published — first-mover available.

## Why post-MVP

Stoa v0.1 ships injection at M5; without injection live, the agent can't actually use the harvested memory. Running MemoryArena pre-M5 would measure nothing.

## Implementation

Adapter: `crates/stoa-bench/src/memory_arena.rs` (post-v0.1). Adapter shells out to whatever agent harness is in use; Stoa's contribution is the memory + injection layer.
