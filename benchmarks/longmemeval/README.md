# LongMemEval

**Status**: v0.1 required. Already committed in [PRODUCT.md](../../PRODUCT.md) and [ROADMAP.md](../../ROADMAP.md) M4/M6.

## Source

[LongMemEval: Benchmarking Chat Assistants on Long-Term Interactive Memory](https://arxiv.org/abs/2410.10813) — Wu et al., 2024.

## What it measures

Five core abilities over long multi-session histories: information extraction, multi-session reasoning, knowledge updates, temporal reasoning, abstention. 500 hand-curated questions, sessions average ~115K tokens of history.

## Why for Stoa

Industry baseline for chat-assistant memory. Mempalace's headline number came from this benchmark (and was misleading — see [PRODUCT.md §Why we'll win](../../PRODUCT.md) point 3). Stoa publishes recall@k from day one against a fixed corpus to refute the same pattern.

## Cost

Corpus is public (HuggingFace). 500 questions through a judge LLM ≈ $30–60 per run at Haiku/4o-mini rates. No GPU required for the recall layer.

## Stoa-specific protocol

- Default backend: `LocalChromaSqliteBackend` (ChromaDB + SQLite FTS5 + KG).
- Metrics published: `recall@1`, `recall@5`, `recall@10` per question category.
- Pin the scorer commit hash in `results/<version>-<backend>-longmemeval.md`.
- Backend swaps re-run against the same corpus + scorer; results published before the swap merges.

## Gameability notes

- Knowledge-update category gameable by overfitting injection prompts to the LongMemEval question style. Mitigation: the same backend code must pass MemoryAgentBench's FactConsolidation sub-task without prompt-tuning per benchmark.
- Long-context backbones (Claude 4.x with 1M context) can score competitively without any memory system. Stoa's headline must be **delta vs no-memory baseline at the same backbone**, not absolute.

## Implementation

Runner: `crates/stoa-bench` (M5+). Run with `just bench` once implementation lands. Defaults to LocalChromaSqliteBackend per the workspace config.
