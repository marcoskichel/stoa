# STaRK

**Status**: Post-MVP. Gated on a non-stub KG layer (M11+).

## Source

[STaRK: Benchmarking LLM Retrieval on Semi-structured Knowledge Bases](https://arxiv.org/abs/2404.13207) — NeurIPS Datasets & Benchmarks 2024. SNAP (Stanford), same group behind LoCoMo.

## What it measures

Semi-structured retrieval combining **entity-graph relations with textual descriptions**. Three knowledge bases:

- Amazon product graph
- Academic citation graph
- PrimeKG (precision medicine)

Queries require joint reasoning over text + graph structure. Pure embedding retrieval degrades; BM25+graph traversal lifts.

## Why for Stoa

The KG layer in [ARCHITECTURE.md §6.1](../../ARCHITECTURE.md) is "lightweight" by design. STaRK measures whether BM25+embedding alone answers queries that require entity-relation traversal, or whether the KG traversal adds measurable lift. This is the design-validation benchmark for the BM25 + vector + KG fusion choice.

The academic-citation sub-corpus (entities = papers, relations = citations / authors) is the closest analog to Stoa's code-entities / call-graph structure. Run that sub-corpus; skip the product-search one.

## Why post-MVP

KG is a stub in M1. Same constraint as AMA-Bench — without a real KG, scores measure only the BM25+embedding layer, which BEIR already covers more thoroughly.

## Cost

- All three KBs public.
- Standard retrieval metrics (Recall@k, MRR).
- No LLM judge.
- Cheap to run on a single sub-corpus.

## Gameability notes

- Not a memory benchmark. Pure retrieval-over-heterogeneous-KG.
- Relevance to Stoa is narrow — validates the BM25+KG retrieval design only, not the memory consolidation pipeline.
- No memory vendor publishes STaRK numbers.

## Implementation

Adapter: `crates/stoa-bench/src/stark.rs` (post-M11). Run only the academic-citation sub-corpus by default.
