# MEMTRACK

**Status**: v0.1 required.

## Source

[MEMTRACK: Evaluating Long-Horizon Memory Tracking in Software Engineering Workflows](https://arxiv.org/abs/2510.01353) — Deshpande et al., Patronus AI; NeurIPS SEA Workshop 2025. [Patronus announcement post](https://www.patronus.ai/blog/memtrack).

## What it measures

47 expert-curated scenarios simulating a software organization's event timeline across Slack, Linear, and Git. Three metrics:

1. **Correctness** — did the agent recall the right state.
2. **Efficiency** — number of tool calls used.
3. **Redundancy** — unnecessary re-fetching.

Scenarios include cross-platform dependencies and explicit conflicts (e.g. Linear ticket status contradicts a Slack message). Best published score: 60% correctness (GPT-5).

## Why for Stoa

Structurally the closest benchmark to Stoa's actual use case. Claude Code users generate events across sessions in multiple modalities (code edits, terminal output, conversation turns). The cross-modality conflict resolution is exactly what session HARVEST → KG update is supposed to handle. The Redundancy metric is a proxy for injection efficiency (token-to-utilization ratio).

## Cost

- 47 scenarios — one-afternoon run.
- CC-BY-4.0.
- LLM judge required for Correctness; full run < $50.
- No GPU.

## Gameability notes

- 47 data points is too small for sub-category statistical confidence. Report aggregate only.
- High curation quality but tiny size means single prompt-engineering wins swing the score. Pin prompt templates in `results/`.
- No standard leaderboard yet — first-mover for memory systems.

## Implementation

Adapter: `crates/stoa-bench/src/memtrack.rs` (M5+). Patronus repo provides the scenario JSON + judge prompt; harness is straightforward.
