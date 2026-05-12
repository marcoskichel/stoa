# BEAM (Beyond a Million Tokens)

**Status**: v0.1 required.

## Source

[BEAM: Beyond a Million Tokens](https://arxiv.org/abs/2510.27246) — ICLR 2026. Hindsight leaderboard post: [hindsight.vectorize.io/blog/2026/04/02/beam-sota](https://hindsight.vectorize.io/blog/2026/04/02/beam-sota).

## What it measures

100 auto-generated coherent conversations with 2,000 validated questions, scaled to **128K / 500K / 1M / 10M tokens**. Ten memory abilities including contradiction resolution, event ordering, instruction following, knowledge update. Multi-scale design — each tier publishes separately.

## Why for Stoa

The only benchmark that stress-tests at scales where context-stuffing is physically impossible (10M tokens ≈ a year of daily sessions). The BM25+embedding recall layer is the only infrastructure that enables non-trivial 10M performance — so BEAM is the benchmark that directly differentiates the recall architecture rather than the summarization layer.

Public leaderboard already stratifies the space:
- Hindsight: 73.9% (1M) / 64.1% (10M)
- Mem0: 48.6% (10M)
- Others trail far behind

Coding-domain sub-corpus exists within BEAM and is the relevant slice for Stoa's marketing.

## Cost

- Dataset synthetic + public.
- Rule-based scoring for most tasks → no expensive judge.
- 128K + 1M tier: low cost (recall-layer-only eval, no long-context inference).
- 10M tier: expensive only if running end-to-end QA against a long-context backbone. Stoa's primary metric is recall@k, which scales linearly with chunk count — manageable.

Run the 128K + 1M tiers for the v0.1 release card; 10M tier optional but high-impact for positioning.

## Gameability notes

- Synthetically generated conversations may not reflect real user behavior.
- 128K tier already saturated by strong long-context backbones — Stoa's headline should be 1M and 10M, not 128K.
- 10M tier is the differentiator: not yet saturated by anyone.

## Implementation

Adapter: `crates/stoa-bench/src/beam.rs` (M5+). Each tier produces a separate `results/` file.
