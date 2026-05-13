# BEAM (Beyond a Million Tokens)

**Status**: v0.1 required.

## Source

- Paper: [BEAM: Beyond a Million Tokens](https://arxiv.org/abs/2510.27246) — ICLR 2026.
- Datasets: [`Mohammadta/BEAM`](https://huggingface.co/datasets/Mohammadta/BEAM) (128K / 500K / 1M) and [`Mohammadta/BEAM-10M`](https://huggingface.co/datasets/Mohammadta/BEAM-10M) (10M tier — separate).
- Repo: [github.com/mohammadtavakoli78/BEAM](https://github.com/mohammadtavakoli78/BEAM).
- Leaderboard: [hindsight.vectorize.io/blog/2026/04/02/beam-sota](https://hindsight.vectorize.io/blog/2026/04/02/beam-sota).

## What it measures

100 auto-generated coherent conversations with 2,000 validated questions distributed across **128K / 500K / 1M / 10M token tiers**. Tier distribution: 20 conversations at 128K, 35 at 500K, 35 at 1M, 10 at 10M. 20 questions per conversation.

Each question carries one of these ten **exact** memory ability labels:

`abstention`, `contradiction_resolution`, `event_ordering`, `information_extraction`, `instruction_following`, `knowledge_update`, `multi_hop_reasoning`, `preference_following`, `summarization`, `temporal_reasoning`.

Scoring is nugget-based (0 / 0.5 / 1 per atomic semantic unit). Answers link back to source dialogue turns.

## Why for Stoa

The only benchmark that stress-tests at scales where context-stuffing is physically impossible (10M tokens ≈ a year of daily sessions). The BM25+embedding recall layer is the only infrastructure that enables non-trivial 10M performance — so BEAM is the benchmark that directly differentiates the recall architecture rather than the summarization layer.

Public leaderboard ([snapshot 2026-04-02](https://hindsight.vectorize.io/blog/2026/04/02/beam-sota)):

| Tier | Hindsight | Honcho | LIGHT baseline | RAG baseline |
|---|---|---|---|---|
| 100K | 73.4% | 63.0% | 35.8% | 32.3% |
| 500K | 71.1% | 64.9% | 35.9% | 33.0% |
| 1M | 73.9% | 63.1% | 33.6% | 30.7% |
| 10M | 64.1% | 40.6% | 26.6% | 24.9% |

Mem0 claims 64.1% (1M) / 48.6% (10M) on its own research page but does not appear on the Hindsight leaderboard; the 64.1% at 1M coincidentally equals Hindsight's score at the same tier, so treat as self-reported until independently verified.

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
