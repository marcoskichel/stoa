# Memora + FAMA

**Status**: Post-MVP. Gated on M12 (crystallize + invalidation, v0.3 per [ARCHITECTURE.md §14](../../ARCHITECTURE.md) tier 12).

## Source

[From Recall to Forgetting: A New Benchmark for Long-Term Memory Agents](https://arxiv.org/abs/2604.20006) — April 2026. Introduces the **Memora** dataset + the **FAMA** scoring metric.

## What it measures

10 professional personas, ~1,100 conversations spanning weekly / monthly / quarterly timescales, ~600 evaluation questions. Tasks: remembering, reasoning, recommending.

**FAMA** rewards the system only when it has correctly **invalidated** the previous version of a claim — not just when it retrieves the latest one. Standard recall metrics reward any retrieval of relevant text; FAMA penalizes use of obsolete memory.

The original [PRODUCT.md](../../PRODUCT.md) already cites Memora's finding that memory systems lose **18–32% accuracy over weeks/months** when they only add facts and never retire stale ones. FAMA is the scoring metric that operationalizes this.

## Why for Stoa

The cleanest test of [the crystallize + supersession pipeline](../../ARCHITECTURE.md). Stoa's nightly crystallize produces both new synthesis drafts and supersession proposals; FAMA scores whether the supersession proposals actually fire correctly.

## Why post-MVP

The crystallize + invalidation pass lands at M12 (v0.3). Pre-M12, Stoa has no supersession path; FAMA would score 0 on the metric it exists to measure.

## Cost

- CC-BY-4.0 corpus.
- **Multi-judge protocol** (3 LLMs per criterion) — notably more expensive than single-judge benchmarks.
- ~$300–600 per full run using Haiku / Flash / 4o-mini judges.
- No GPU.

## Gameability notes

- Very new (April 2026), no independent replications yet.
- Quarterly-scale conversations are LLM-synthesized, not real user data.
- No vendor comparison baseline yet — adopting early is a first-mover position.

## Implementation

Adapter: `crates/stoa-bench/src/fama.rs` (M12+). Adapter must drive the full crystallize → supersession → recall loop; pre-crystallize results are meaningless.
