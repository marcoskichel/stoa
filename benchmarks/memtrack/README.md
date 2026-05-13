# MEMTRACK

**Status**: v0.1 required.

## Source

- Paper: [MEMTRACK](https://arxiv.org/abs/2510.01353) — Deshpande, Gangal et al., Patronus AI; NeurIPS 2025 SEA Workshop.
- Blog: [Patronus announcement post](https://www.patronus.ai/blog/memtrack).
- Dataset: **No HuggingFace card published.** Distributed via Google Drive (file ID `1ymMXmOIhCUcwC1WKOW8kioZgeYyrt-qe`). See `benchmarks/corpus/memtrack.sh` — requires `gdown`.
- Scorer: **No public scorer repo.** Implement from paper Section IV: Correctness via partial/approximate match + LLM-as-judge; Efficiency = tool-call count; Redundancy = unnecessary re-fetches.

## What it measures

47 expert-curated instances simulating a software organization's event timeline across Slack, Linear, and Gitea (self-hosted Git in a Docker container — not GitHub). Three metrics:

1. **Correctness** — did the agent recall the right state.
2. **Efficiency** — number of tool calls used.
3. **Redundancy** — unnecessary re-fetching.

Scenarios include cross-platform dependencies and explicit conflicts (e.g. Linear ticket status contradicts a Slack message). Best published score: GPT-5 at 60% Correctness. The paper explicitly notes memory backends (Mem0, Zep) do not significantly improve over the no-memory baseline.

## Schema

Each instance is `(timeline, [(Q1, A1), ..., (Qn, An)])` where the timeline is a sequence of platform-typed events. Average 3.2 questions per instance (max 5). Average 39.9 events per instance (max 115). Average timeline span 878 hours.

Event fields are platform-heterogeneous:

| Platform | Required fields | Notes |
|---|---|---|
| `slack` | `timestamp`, `channel`, `sender`, `message` | Flat message events |
| `linear` | `timestamp`, `title`, `description`, `team`, `priority`, `lead`, `attached_resources` | Structured issue events |
| `gitea` | `timestamp`, `repo`, `event_type`, `author`, `message` | Filesystem-backed via Gitea container |

Questions are injected **sequentially** — the agent cannot see all `n` questions upfront.

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
