# Benchmarks

Reproducible evaluation suite for Stoa's memory + recall pipeline.

## What lives here

```
benchmarks/
├── README.md                 # this file — index + plan
├── spike-m0.md               # M0 stack-validation report (frozen)
├── spike-m0/                 # M0 spike code (excluded from Cargo workspace)
├── corpus/                   # shared download scripts (data gitignored)
├── results/                  # published per-backend results (CI-populated)
├── longmemeval/              # LongMemEval runner (v0.1 required)
├── memory-agent-bench/       # MemoryAgentBench (v0.1 required)
├── memtrack/                 # MEMTRACK (v0.1 required)
├── beam/                     # BEAM scale benchmark (v0.1 required)
├── agent-leak/               # AgentLeak PII channel benchmark (v0.1 required)
├── mteb-retrieval/           # MTEB/BEIR subset (component check, v0.1)
├── memory-arena/             # MemoryArena agentic tasks (post-MVP, M3+)
├── fama/                     # Memora + FAMA staleness scorer (post-MVP)
├── ama-bench/                # AMA-Bench causality (post-MVP, KG-dependent)
├── swe-bench-cl/             # SWE-Bench-CL continual learning (post-MVP)
└── stark/                    # STaRK hybrid text+KG retrieval (post-MVP)
```

Each leaf directory has its own `README.md` with: source paper, what it measures, why it matters for Stoa specifically, cost envelope, gameability notes, and the milestone that gates a real implementation. Until a benchmark is wired up, its directory holds only the README — no fake harness, no placeholder data.

Common runner code lives in `crates/stoa-bench` (M5+). Each benchmark wires a thin adapter that produces results matching the schema in `results/`.

## Suite design

Two non-negotiable rules from [PRODUCT.md](../PRODUCT.md) and [ARCHITECTURE.md §6.1](../ARCHITECTURE.md):

1. **Fixed test corpus.** Every benchmark pins its dataset + scorer version. No swapping the corpus to chase a number.
2. **Every `RecallBackend` adapter publishes against the same suite.** Backend swaps are quality-gated. A silent recall regression on backend swap is the single failure mode the suite exists to prevent.

Results land in `results/<version>-<backend>-<benchmark>.md` (e.g. `results/v0.1-local-chroma-sqlite-longmemeval.md`). CI publishes; manual edits forbidden.

## v0.1 suite (MVP)

Run as the release card for v0.1. Total cost budget: < $500/run on the full suite. All required for the v0.1 ship per [ROADMAP.md](../ROADMAP.md) M6 exit criteria.

| Benchmark | Measures | Cost | First-mover vs peers |
|---|---|---|---|
| [longmemeval](./longmemeval/) | Long-term multi-session recall + reasoning | Low | All major peers published |
| [memory-agent-bench](./memory-agent-bench/) | Selective forgetting + test-time learning + retrieval. FactConsolidation directly exercises crystallize/supersession | Low | Mem0, Zep, Letta, MIRIX, Cognee |
| [memtrack](./memtrack/) | Multi-platform event-timeline state tracking (Slack/Linear/Git) with conflict resolution. 47 expert scenarios | Very low | Yes — no memory vendor published |
| [beam](./beam/) | Recall at 128K/500K/1M/10M tokens. Stresses the recall architecture where context-stuffing is impossible | Medium | Mem0, Hindsight, Evermind |
| [agent-leak](./agent-leak/) | 32-class PII leak taxonomy across 7 channels incl. shared-memory channel. Validates redaction + MINJA delimiters | Low–med | Yes — no memory vendor published |
| [mteb-retrieval](./mteb-retrieval/) | Embedding-component check on BEIR subset. Internal engineering decision tool, not marketing | Near-zero | All embedding vendors publish |

## Post-MVP (M3+)

Wired up after the milestones that gate their measurement targets:

| Benchmark | Gated on | Why later |
|---|---|---|
| [memory-arena](./memory-arena/) | M5 injection live + budget for $500–2K/run | End-to-end agentic task completion. Quarterly cadence at best |
| [fama](./fama/) | M12 crystallize + invalidation (v0.3) | Measures supersession of stale claims. No invalidation path = no signal |
| [ama-bench](./ama-bench/) | M11 KG layer non-stub | Tests causality-aware memory. Need real KG edges to score |
| [swe-bench-cl](./swe-bench-cl/) | Upstream harness completion + $1.5–4K/run | Coding continual learning. Closest fit; harness was incomplete at publication |
| [stark](./stark/) | M11 KG live | Hybrid text+KG retrieval. Validates BM25+graph lift |

## Explicitly out of scope

Surveyed and cut. Do not add without a written reason that overturns these:

- **EMERGE** (Wikidata-scale KG population) — wrong scope. Stoa builds a per-user lightweight KG, not a public-KG triple extractor.
- **CRUD-RAG** — Chinese-language only.
- **EvolMem / EverMemBench / EngramaBench** — yet-another-multi-session benchmarks with no published peer numbers. LongMemEval, LoCoMo, BEAM, MemoryAgentBench already saturate this category.
- **FiFA / Forgetful-but-Faithful** — needs explicit user-controlled forgetting policies. Stoa v0.1 deletes via supersession only; revisit at v0.2.
- **KGQA suite** (WebQuestions, QALD, LC-QuAD) — SPARQL over public KGs. Wrong shape: Stoa's KG is private + session-derived.
- **RAGBench / RAGAS** — instrument token-utilization first-party in Stoa's injection path. Borrow the TRACe definition; skip the corpus.

## Discipline (load-bearing)

From [ROADMAP.md §Cross-cutting tracks](../ROADMAP.md):

- No test-corpus changes without re-running prior backends and re-publishing.
- No headline numbers from a corpus different from the public one.
- No public release without published numbers for **every** v0.1-tier benchmark.
- LoCoMo trap: if added later, pin scorer version + document `created_at` usage. Zep's self-reported 84% → corrected 58.44% is the cautionary tale.

## Adding a new benchmark

1. Create `benchmarks/<name>/README.md` covering: source, what it measures, Stoa relevance, cost, gameability, gating milestone.
2. If MVP-tier: amend this file's table + [ROADMAP.md](../ROADMAP.md) M4/M6 exit criteria + [ARCHITECTURE.md §14](../ARCHITECTURE.md) tier 6.
3. Adapter lives in `crates/stoa-bench/src/<name>.rs`.
4. First result lands in `results/<version>-<backend>-<name>.md` via CI.
