# MemoryAgentBench

**Status**: v0.1 required.

## Source

[MemoryAgentBench: A Benchmark for Agentic Memory in Long-Context Reasoning](https://arxiv.org/abs/2507.05257) — Hu, Wang, McAuley; ICLR 2026. Dataset on HuggingFace, MIT/CC-BY-4.0.

## What it measures

Four competencies across 2,071 questions, contexts spanning 103K–1.44M tokens:

1. **Accurate retrieval** — does the system find the right snippet.
2. **Test-time learning** — does it adapt to new domain info appearing mid-session.
3. **Long-range understanding** — does it answer questions requiring synthesis across the whole history.
4. **Selective forgetting** (FactConsolidation-SH / FactConsolidation-MH) — does it correctly update memory state when a prior claim is invalidated.

Feed design is **incremental** — chunks arrive one at a time, mirroring the queue → worker pattern.

## Why for Stoa

The FactConsolidation sub-tasks are the most direct test of the [crystallize + invalidation pipeline](../../ARCHITECTURE.md). Selective forgetting is the named failure mode of FAMA at smaller scale; here it's a built-in score category. The incremental feed maps one-to-one onto Stoa's `.stoa/queue.db` drain pattern, so the benchmark exercises the same control flow as production.

## Cost

- Dataset: HuggingFace, MIT/CC-BY-4.0.
- 2,071 questions × judge LLM ≈ $30–60 at Haiku/4o-mini rates.
- No GPU required.
- Five commercial memory systems (MIRIX, MemGPT/Letta, Mem0, Cognee, Zep) provide direct comparison baselines from the paper.

## Gameability notes

- Very new (July 2025), no contamination analysis published.
- 2,071 questions is small enough that prompt-engineering one sub-task can swing aggregate scores. Mitigation: publish per-sub-task scores, not just aggregate.

## Implementation

Adapter: `crates/stoa-bench/src/memory_agent_bench.rs` (M5+). The incremental-feed harness is not provided upstream; Stoa writes its own and contributes upstream if useful.
