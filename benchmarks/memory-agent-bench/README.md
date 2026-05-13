# MemoryAgentBench

**Status**: v0.1 required.

## Source

- Paper: [Evaluating Memory in LLM Agents via Incremental Multi-Turn Interactions](https://arxiv.org/abs/2507.05257) — HUST AI lab, ICLR 2026.
- Dataset: [`ai-hyz/MemoryAgentBench`](https://huggingface.co/datasets/ai-hyz/MemoryAgentBench) on HuggingFace.
- Scorer: [github.com/HUST-AI-HYZ/MemoryAgentBench](https://github.com/HUST-AI-HYZ/MemoryAgentBench).

## What it measures

Four top-level HuggingFace splits map 1:1 to four competencies:

1. **`Accurate_Retrieval`** — sub-tasks: SH-Doc QA, MH-Doc QA, LongMemEval, EventQA.
2. **`Test_Time_Learning`** — sub-tasks: BANKING77, CLINC150, TREC-Coarse, TREC-Fine, NLU, Movie Recommendation.
3. **`Long_Range_Understanding`** — sub-tasks: Novel Summarization (InfBench-Sum), Detective QA.
4. **`Conflict_Resolution`** — sub-tasks: **FactConsolidation-SH** (single-hop), **FactConsolidation-MH** (multi-hop). The paper's prose name for this competency is "selective forgetting"; the dataset split label is `Conflict_Resolution`.

Feed design is **incremental** — chunks arrive one at a time, mirroring Stoa's queue → worker pattern.

## Schema

Each row inside a split has fields: `context` (string), `questions` (list[string]), `answers` (list[string]), `metadata` (dict with `question_types`, `qa_pair_ids`, `source`, ...). The `metadata.question_types` list carries the per-sub-task label (e.g. `FactConsolidation-SH`).

## Why for Stoa

The FactConsolidation sub-tasks are the most direct test of the [crystallize + invalidation pipeline](../../ARCHITECTURE.md). Selective forgetting is the named failure mode of FAMA at smaller scale; here it's a built-in score category. The incremental feed maps one-to-one onto Stoa's `.stoa/queue.db` drain pattern, so the benchmark exercises the same control flow as production.

## Cost

- Dataset: HuggingFace, MIT/CC-BY-4.0.
- 2,071 questions × judge LLM ≈ $30–60 at Haiku/4o-mini rates.
- No GPU required.

## Published peer scores (Conflict_Resolution split, Table 3, accuracy %)

| Agent | FC-SH | FC-MH |
|---|---|---|
| Cognee | 28 | 3 |
| Mem0 | 18 | 2 |
| MIRIX | 14 | 2 |
| Zep | 7 | 3 |

Multi-hop collapses across the board (max 7% on any agent). Long-context GPT-4o reaches ~60% on SH but also fails on MH. The Conflict_Resolution split is the hardest of the four and the most directly tied to Stoa's crystallize + invalidation pipeline.

## Gameability notes

- Very new (July 2025), no contamination analysis published.
- 2,071 questions is small enough that prompt-engineering one sub-task can swing aggregate scores. Mitigation: publish per-sub-task scores, not just aggregate.

## Implementation

Adapter: `crates/stoa-bench/src/memory_agent_bench.rs` (M5+). The incremental-feed harness is not provided upstream; Stoa writes its own and contributes upstream if useful.
