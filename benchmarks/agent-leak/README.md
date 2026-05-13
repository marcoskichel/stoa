# AgentLeak

**Status**: v0.1 required.

## Source

- Paper: [AgentLeak: A Full-Stack Benchmark for Privacy Leakage in Multi-Agent LLM Systems](https://arxiv.org/abs/2602.11510) — Feb 2026.
- Dataset: [`humain2/AgentLeak`](https://huggingface.co/datasets/humain2/AgentLeak) on HuggingFace.
- Repo: [github.com/Privatris/AgentLeak](https://github.com/Privatris/AgentLeak).

## What it measures

1,000 scenarios across healthcare, finance, legal, corporate domains. **32-class attack taxonomy** organized into 6 families: F1 Prompt/Instruction, F2 Indirect/Tool-Surface, F3 Memory/Persistence, F4 Multi-Agent Coordination, F5 Reasoning/CoT, F6 Evasion/Obfuscation. Per-class names are not publicly enumerated in the paper.

Seven leak channels per paper Section III-B (**canonical taxonomy** — earlier Stoa docs swapped C4/C6):

- C1 — direct output
- C2 — inter-agent messages
- C3 — tool arguments
- **C4 — tool output** (data returned from tool invocations)
- **C5 — shared memory / agent state** ← direct attack vector through Stoa's recall layer
- **C6 — system logs / telemetry / audit**
- C7 — environment / config

Key finding: multi-agent configurations reduce per-channel **output** leakage (27.2% vs 43.2%) but raise **total system** exposure to 68.9% when internal channels are counted.

## Why for Stoa

[ARCHITECTURE.md §6.2 + §8](../../ARCHITECTURE.md) commits to PII redaction in the capture path and MINJA-resistant XML delimiters in injection. Both claims need adversarial validation, not just unit tests. AgentLeak's C5 channel is the attack surface through Stoa's recall layer specifically — running it operationalizes the privacy posture into a number.

Distinguishes Stoa from Mem0 / Zep / Letta, none of whom publish PII-defense numbers.

## Cost

- 1,000 scenarios with classification judge.
- HuggingFace dataset, CC-BY-style.
- ~$100–300 at Haiku/4o-mini rates plus one evening of harness setup.
- No GPU.

## Gameability notes

- Very new; taxonomy completeness not independently validated.
- Adversarial recall metric: high score reflects absence of **known** attack classes only.
- Upstream tests against LangChain, CrewAI, AutoGPT, MetaGPT — Stoa's hook architecture is none of these, so the adapter must map the attack scenarios onto Stoa's surfaces (Stop hook, capture worker, SessionStart injection).

## Implementation

Adapter: `crates/stoa-bench/src/agent_leak.rs` (M5+). Adapter must explicitly cover at minimum:

- C1 via `stoa query` output
- C5 via `stoa inject log` and the recall layer (shared memory)
- C6 via `.stoa/audit.log` (system logs / telemetry)
