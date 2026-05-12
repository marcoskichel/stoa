# SWE-Bench-CL

**Status**: Post-MVP. Gated on upstream harness completion + budget for $1.5–4K/run.

## Source

[SWE-Bench-CL: Continual Learning on SWE-Bench](https://arxiv.org/abs/2507.00014) — June 2025.

## What it measures

273 real GitHub issues from 8 Python repositories organized into **chronologically ordered sequences**. Metrics:

- Average accuracy.
- **Forgetting** (backward transfer) — does solving issue T+N degrade performance on issue T.
- **Forward transfer** — does solving issue T help with issue T+N.
- **CL-F1** — balanced plasticity vs stability.

Directly tests whether a coding agent that solved an issue in repository X at time T can leverage that experience when solving a related issue at time T+N.

## Why for Stoa

The closest published benchmark to Stoa's coding-agent thesis. The wiki should cause **forward transfer to increase** and **forgetting to decrease** vs a baseline agent.

## Why post-MVP

- Upstream harness was incomplete at publication; authors reported "ongoing" experiments and compatibility issues.
- 273 tasks × full SWE-Bench solve pipeline = $5–15 per task = $1,500–4,000 per full run. Not a monthly cadence benchmark.
- 8 repos are mostly data-science / web (Django, scikit-learn, matplotlib) — reasonable overlap with coding agents but not Rust / systems work.

Wait for upstream harness to stabilize. Re-evaluate at v0.2.

## Cost

- Public corpus (inherited from SWE-Bench).
- Per-task LLM cost is the dominant line item.
- Optionally limit to a subset (e.g. one repo) for cheaper smoke runs.

## Gameability notes

- CL metrics are novel and not yet calibrated — "forgetting" can be confounded by model stochasticity across runs. Run with seed pinning + multiple seeds.
- No published leaderboard yet.

## Implementation

Adapter: `crates/stoa-bench/src/swe_bench_cl.rs` (post-v0.1). Wraps the upstream SWE-Bench harness with chronological ordering + the CL metrics.
