# MTEB / BEIR retrieval subset

**Status**: v0.1 required. Internal-engineering tool, not headline marketing.

## Source

[MTEB: Massive Text Embedding Benchmark](https://github.com/embeddings-benchmark/mteb) — Muennighoff et al., 2022; maintained through 2026. Underlying retrieval corpora are [BEIR](https://github.com/beir-cellar/beir).

## What it measures

Zero-shot retrieval across 15 heterogeneous corpora (MS MARCO, TREC-COVID, NQ, HotpotQA, etc.). Metric: NDCG@10. Tests whether an embedding model generalizes across domain shifts.

## Why for Stoa

Stoa's `RecallBackend` is swappable, but the **embedding model inside the backend** is the dominant factor in recall quality on prose. MTEB/BEIR is the only standardized way to isolate the embedding model's contribution from the surrounding memory architecture.

Use case: when evaluating a new embedding model or updating the default, run a BEIR subset (5–8 datasets) to gate the decision. This is **not** a public marketing benchmark — it's the internal control gate for embedding swaps.

## Cost

- Near-zero.
- All datasets public.
- Fully automated evaluation, no LLM judge calls.
- Embedding+retrieval only — runs in hours on CPU for the BEIR subset.

## Gameability notes

- Heavily saturated (top models cluster around NDCG@10 = 0.55–0.60).
- Fine-tuning on a BEIR training split disqualifies zero-shot status. Stoa never trains, only embeds.
- Tells you **nothing** about memory consolidation, update, or multi-session behavior. Pure component-level recall quality.

## Implementation

Adapter: `crates/stoa-bench/src/mteb_retrieval.rs` (M5+). Use the BEIR subset (no need for the full MTEB suite). Run on every embedding-default change; gate the change on no NDCG@10 regression on the subset.

Result publishing: write to `results/`, but **do not** include in the v0.1 public release card. It's an engineering control, not a positioning artifact.
