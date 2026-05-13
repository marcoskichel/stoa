# LongMemEval

**Status**: v0.1 required. Already committed in [PRODUCT.md](../../PRODUCT.md) and [ROADMAP.md](../../ROADMAP.md) M4/M6.

## Source

- Paper: [LongMemEval: Benchmarking Chat Assistants on Long-Term Interactive Memory](https://arxiv.org/abs/2410.10813) — Wu et al., ICLR 2025.
- Dataset: [`xiaowu0162/longmemeval-cleaned`](https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned) on HuggingFace (cleaned split — fixes `longmemeval_oracle.json` load error in the original `xiaowu0162/longmemeval`).
- Scorer: [github.com/xiaowu0162/LongMemEval](https://github.com/xiaowu0162/LongMemEval) → `src/evaluation/evaluate_qa.py` (GPT-4o-as-judge). No release tags — pin to a main HEAD SHA in `results/`.

## What it measures

Five core abilities over long multi-session histories: information extraction, multi-session reasoning, knowledge updates, temporal reasoning, abstention. 500 hand-curated questions, sessions average ~115K tokens of history.

## Schema

The dataset is a flat JSON array of question records with these exact fields:

- `question_id` (string)
- `question_type` — one of `single-session-user`, `single-session-assistant`, `single-session-preference`, `multi-session`, `temporal-reasoning`, `knowledge-update`. Abstention is encoded as an `_abs` suffix on any of these (e.g. `single-session-user_abs`).
- `question` / `answer` (strings)
- `question_date`, `haystack_dates` (ISO date strings)
- `haystack_session_ids` (list of strings)
- `haystack_sessions` (list of sessions; each session is a list of turns with `role` / `content` and an optional `has_answer: true` flag on evidence turns)
- `answer_session_ids` (list of strings — empty for abstention)

## Why for Stoa

Industry baseline for chat-assistant memory. Mempalace's headline number came from this benchmark (and was misleading — see [PRODUCT.md §Why we'll win](../../PRODUCT.md) point 3). Stoa publishes recall@k from day one against a fixed corpus to refute the same pattern.

## Cost

Corpus is public (HuggingFace). 500 questions through a judge LLM ≈ $30–60 per run at Haiku/4o-mini rates. No GPU required for the recall layer.

## Stoa-specific protocol

- Default backend: `LocalChromaSqliteBackend` (ChromaDB + SQLite FTS5 + KG).
- Metrics published: `recall@1`, `recall@5`, `recall@10` per question category.
- Pin the scorer commit hash in `results/<version>-<backend>-longmemeval.md`.
- Backend swaps re-run against the same corpus + scorer; results published before the swap merges.

## Published peer scores (on LongMemEval_S, GPT-4o backbone)

| System | Score | Source | Notes |
|---|---|---|---|
| Oracle (full context, GPT-4o) | 82.4% | Wu et al. 2024 | Upper bound for the backbone |
| Mastra Observational Memory | 84.2% | mastra.ai/research | Self-reported |
| Emergence AI (EmergenceMem) | 86% | emergence.ai blog | Self-reported, beats Oracle |
| Hindsight (Gemini-3) | 91.4% | vectorize-io/hindsight-benchmarks | Different backbone |
| ByteRover 2.0 | 92.2–92.8% | byterover blog | Self-reported |
| Zep / Graphiti | 71.2% (self) / **63.8% (independent)** | atlan.com/know/zep-vs-mem0 | 7-point gap on independent reproduction |
| Mem0 | ~49% | Zep comparative paper | Independent |

Vendor self-reports diverge from independent reproductions. Treat any unreproduced number as self-reported.

## Gameability notes

- Knowledge-update category gameable by overfitting injection prompts to the LongMemEval question style. Mitigation: the same backend code must pass MemoryAgentBench's FactConsolidation sub-task without prompt-tuning per benchmark.
- Long-context backbones (Claude 4.x with 1M context) can score competitively without any memory system. Stoa's headline must be **delta vs no-memory baseline at the same backbone**, not absolute.

## Implementation

Runner: `crates/stoa-bench` (M5+). Run with `just bench` once implementation lands. Defaults to LocalChromaSqliteBackend per the workspace config.
