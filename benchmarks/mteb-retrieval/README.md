# MTEB / BEIR retrieval subset

**Status**: v0.1 required. Internal-engineering tool, not headline marketing.

## Source

- Benchmark: [MTEB](https://github.com/embeddings-benchmark/mteb) — Muennighoff et al., 2022; maintained through 2026. Underlying retrieval corpora are [BEIR](https://github.com/beir-cellar/beir).
- Datasets used by Stoa: [`BeIR/scifact`](https://huggingface.co/datasets/BeIR/scifact), [`BeIR/nfcorpus`](https://huggingface.co/datasets/BeIR/nfcorpus), [`BeIR/fiqa`](https://huggingface.co/datasets/BeIR/fiqa). Qrels are separate datasets at `BeIR/<name>-qrels`.
- Scorer: `mteb` PyPI package (`embeddings-benchmark/mteb`), latest 2.12.30 (2026-04-25). Pin a specific version in `results/`. Implementation: `mteb.evaluation.evaluators.RetrievalEvaluator`.
- Leaderboard: [huggingface.co/spaces/mteb/leaderboard](https://huggingface.co/spaces/mteb/leaderboard).

## What it measures

Zero-shot retrieval across 15 heterogeneous corpora (MS MARCO, TREC-COVID, NQ, HotpotQA, etc.). Headline metric: **NDCG@10**. Also reports NDCG@1/5/100, MAP@100, Recall@1/10/100, Precision@1/10, MRR@10.

## Schema (BEIR canonical)

- Corpus rows: `_id`, `title` (often empty), `text`. The text body is in `text`, not `title`.
- Query rows: `_id`, `title` (typically empty), `text`. The actual query is in `text`.
- Qrels: separate HF dataset (`BeIR/<name>-qrels`) keyed `{query_id: {doc_id: int}}`. Relevance is 0/1 for SciFact/NFCorpus; 0–4 graded for some others.

## bge-small-en-v1.5 reference scores

Aggregate retrieval NDCG@10 across 15 BEIR tasks: **51.68**. Per-dataset published numbers (pull exact values from the leaderboard before pinning test thresholds):

| Dataset | Approx. NDCG@10 |
|---|---|
| SciFact | 0.69–0.72 |
| FiQA | 0.44–0.47 |
| NFCorpus | 0.33–0.35 |

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
