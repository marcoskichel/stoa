# AMA-Bench

**Status**: Post-MVP. Gated on a non-stub KG layer (M11+).

## Source

[AMA-Bench: Evaluating Long-Horizon Memory for Agentic Applications](https://arxiv.org/html/2602.22769v1) — ICLR 2026 Memory Agent Workshop. Live leaderboard on HuggingFace Spaces.

## What it measures

3,696 QA pairs over real-world agentic trajectories: web navigation, SWE, text-to-SQL, embodied AI, gaming. Plus synthetic BabyAI / TextWorld trajectories.

Tests whether memory systems retain **causally relevant state** from prior agent actions — not just facts from conversations. Best published result: 57.22% (AMA-Agent, the authors' own system).

## Why for Stoa

The paper's headline finding: *"memory systems underperform primarily because they lack causality and objective information."* This is directly applicable to [the lightweight KG layer in ARCHITECTURE.md §6.1](../../ARCHITECTURE.md) — a causality graph between decisions / entities is more useful than pure embedding recall on the agentic-trajectory class of queries.

## Why post-MVP

In M1 the KG layer is a stub. AMA-Bench scores would reflect BM25+embedding only; the benchmark exists to measure the KG contribution. Run after the KG layer has real edges + traversal.

## Cost

- HuggingFace dataset.
- Evaluation uses Qwen3-32B as judge (GPU for local eval, or ~$200–500 in API calls).
- Live leaderboard on HuggingFace Spaces.
- All data open-source, no PII.

## Gameability notes

- Brand new (Feb 2026), no contamination analysis.
- SWE sub-domain within it overlaps with SWE-Bench — high scores may reflect coding agent scaffold quality more than memory quality. Report per-sub-domain, not aggregate.
- 57.22% SOTA — far from saturated.

## Implementation

Adapter: `crates/stoa-bench/src/ama_bench.rs` (post-M11). Adapter must drive KG traversal during retrieval; pure recall mode forfeits the benchmark's signal.
