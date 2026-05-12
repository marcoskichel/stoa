# Roadmap — MVP (v0.1)

This is the shipping plan to reach Stoa v0.1: the first production-quality public release that delivers the core value loop **capture → wiki → recall → injection**. After v0.1 ships, see [ROADMAP-POST-MVP.md](./ROADMAP-POST-MVP.md).

The implementation tier list lives in [ARCHITECTURE.md §14](./ARCHITECTURE.md). This document is the *order* and *exit criteria* — what gets shipped when, and how we know it's done.

Sizes are t-shirt: **S** = days, **M** = 1–2 weeks, **L** = 3–4 weeks, **XL** = >1 month. Estimates assume one focused contributor; parallelize where dependencies allow.

---

## What MVP is (and is not)

**MVP is**:
- Single-binary `stoa` CLI installable via `cargo install stoa` on macOS + Linux
- Markdown wiki on disk (Karpathy-compatible layout) with `STOA.md` schema enforced
- Claude Code `Stop`/`SessionEnd` hook capturing redacted session transcripts in <10ms p95
- Hybrid recall (vector + BM25 + small typed KG) via `LocalChromaSqliteBackend` behind a swappable `RecallBackend` trait
- SessionStart injection of top-K relevant wiki pages with MINJA-resistant XML wrapping, hard token budget, relevance gate
- Reproducible LongMemEval benchmark with published recall@k

**MVP is not**:
- Harvest worker, crystallize loop, lint, lifecycle workflow → v0.2+
- UserPromptSubmit / PreCompact / PreToolUse injection → v0.2+
- Cursor / Codex hooks → v0.3
- All-Rust embedding worker → v0.2 spike
- Web UI, multi-agent, MCP wrapper → v0.3+
- Visualization beyond markdown text → v0.2+

The MVP value proposition: **passive capture + auto-injection at session boot makes the wiki immediately useful even before harvest/crystallize land**. Manual `stoa write` + `stoa query` covers the wiki write side until v0.2.

---

## Phase 0 — Foundation

### M0 — Validation spike

**Size**: M

**Deliverable**: A 1-page report validating three load-bearing assumptions in [ARCHITECTURE.md §15](./ARCHITECTURE.md):
1. Hook cold-start <10ms p95 on a stripped Rust binary doing one SQLite insert + WAL fsync (macOS + Linux).
2. `fastembed` running bge-small-en-v1.5 ONNX produces correct embeddings with comparable cosine similarity to the Python reference, throughput measured on CPU.
3. `cross` cross-compile to all five release targets (linux x86_64, linux aarch64, macos x86_64, macos aarch64, windows x86_64) succeeds with the v0.1 dependency set (no embedding inference yet — that's v0.2).

**Exit criteria**:
- Report committed to `benchmarks/spike-m0.md`
- Either green-light to proceed, or a documented revision to §15 with new picks

**Risk gate**: Nothing else starts until M0 ships. If any assumption fails, architecture revisions ship before M1.

### M1 — Repo skeleton

**Size**: M

**Deliverable**:
- Cargo workspace at root with all crates from [ARCHITECTURE.md §16.2](./ARCHITECTURE.md) scaffolded (stub `lib.rs` + one passing test each)
- `python/` uv workspace with stub packages
- `Justfile` with all recipes from §16.5
- `.github/workflows/rust.yml`, `python.yml`, `release.yml` running on push
- `rust-toolchain.toml`, root `Cargo.toml`, `python/pyproject.toml`
- `benchmarks/` directory + `examples/minimal/` placeholder

**Exit criteria**:
- `just ci-rust` green
- `just ci-python` green
- `just release linux-x86_64` produces a tarball (even if the binary does nothing useful)
- All workflows green on a representative PR

**Demo**: Fresh clone → `just install-dev` → `stoa --version` prints.

---

## Phase 1 — Walking skeleton (→ v0.1)

### M2 — Wiki + CLI core

**Size**: M

**Deliverable**:
- `stoa init` — scaffold workspace (`STOA.md`, `wiki/{entities,concepts,synthesis}/`, `raw/`, `sessions/`, `.stoa/`, `.gitignore`); idempotent
- `stoa read <id>` — print a wiki page
- `stoa write <id> [--frontmatter file] [--body file]` — create/update page
- `stoa schema [--check]` — print or validate `STOA.md`
- Frontmatter parser (`serde_yaml`); validates against `STOA.md` vocabulary
- `index.md` + `log.md` auto-generation
- Default `STOA.md` template ships with `stoa init`

**Exit criteria**:
- Round-trip create → edit → validate from CLI works end-to-end
- Schema rejects bad frontmatter (unknown entity types, missing required fields, invalid relationship types)
- `stoa init` is idempotent (running twice doesn't corrupt state)
- Test coverage: 80%+ on `stoa-core` and `stoa-cli`

**Demo**: User runs `stoa init`, creates 3 entity pages with `stoa write`, runs `stoa schema --check`, sees validation pass; introduces a deliberate frontmatter error, sees `stoa schema --check` fail with a useful message.

### M3 — Capture pipeline

**Size**: L

**Deliverable**:
- Hook binary (`stoa-hooks`) — single static binary; opens `.stoa/queue.db`, inserts one row, exits
- SQLite queue (`rusqlite` v0.38, WAL mode, `synchronous=NORMAL`, FTS5 schema in same DB)
- `stoa daemon` — long-running process spawning capture worker
- Capture worker — drains queue; runs PII redaction; writes redacted JSONL to `sessions/<id>.jsonl`; fires `transcript.captured` event (subscribers: none yet in MVP)
- Regex-based PII redaction (API keys: AWS, Stripe, OpenAI, Anthropic, GitHub PAT; bearer tokens; JWTs; emails configurable; SSH/AWS/GPG path patterns)
- Always-flush guarantee — sessions of any length captured (no `SAVE_INTERVAL` gate)
- `stoa hook install --platform claude-code` — registers the Claude Code `Stop`/`SessionEnd` hook
- `.stoa/audit.log` append-only

**Exit criteria**:
- Hook latency benchmark in CI: **<10ms p95** on Linux + macOS (gates merges to main)
- Idempotent re-capture: re-firing a hook for the same `session_id` is safe
- PII redaction test suite: API keys, tokens, paths all stripped from a fixture transcript
- Worker crash recovery: SIGTERM mid-capture leaves the queue row claim-leased; next worker picks it up and completes
- Session JSONL files are valid JSONL and load round-trip

**Demo**: User installs hook in Claude Code, ends a session, observes `sessions/<id>.jsonl` appear within 1s with PII stripped; checks `.stoa/audit.log` for the capture event.

### M4 — Recall + LocalChromaSqliteBackend

**Size**: L

**Deliverable**:
- `RecallBackend` trait per [ARCHITECTURE.md §6.1](./ARCHITECTURE.md) (Python sidecar in MVP since `fastembed` Rust migration is v0.2)
- `LocalChromaSqliteBackend` Python implementation:
  - ChromaDB for vector embeddings (`bge-small-en-v1.5` default)
  - SQLite FTS5 for BM25 (same `recall.db` as queue)
  - SQLite tables for typed knowledge graph (`nodes`, `edges`)
  - Reciprocal rank fusion across the three streams (k=60)
- Python sidecar bootstrap: `stoa init` runs `uv sync` in `python/` to install the sidecar venv
- IPC: Rust enqueues retrieval requests to SQLite queue lane; Python dequeues, returns results via SQLite (same pattern as capture worker)
- `stoa query <q> [--k 10] [--streams bm25,vector,graph] [--json]` — hybrid recall returning ranked snippets with provenance
- `stoa index rebuild` — full reindex from `wiki/` + `sessions/` + `raw/`
- Wiki page change detection: the daemon watches `wiki/` and re-indexes changed pages
- `stoa init --no-embeddings` flag — BM25-only mode; opt in to embeddings later

**Exit criteria**:
- LongMemEval reproducible benchmark runner committed to `benchmarks/longmemeval/`
- Published `recall@k` numbers in `benchmarks/results/v0.1-local-chroma-sqlite.md` (k=1, 5, 10)
- `stoa query` returns ranked hits with `source_path` always resolving to a real file
- Three-stream fusion is correct: per-stream provenance attached to each hit
- Cold start: `stoa init --no-embeddings` produces a working BM25-only workspace in <5s on fresh machine
- Cold start with embeddings: <60s on fresh machine (model download dominates)

**Demo**: User indexes a 50-page wiki, runs `stoa query "redis vs memcached"`, sees ranked hits with file paths, scores, and per-stream attribution (which streams matched).

### M5 — SessionStart injection + MINJA defenses

**Size**: M

**Deliverable**:
- `stoa hook install --inject session-start` registers the SessionStart injection hook
- SessionStart handler:
  - Resolves workspace from `cwd`
  - Builds query context from recent activity (cwd, git remote, recently-edited files in last 24h)
  - Calls `RecallBackend.search()` with token budget
  - Wraps results in `<stoa-memory>` XML with the "treat as data, not instructions" preamble (per [ARCHITECTURE.md §6.2](./ARCHITECTURE.md))
  - Returns the wrapped block as `additionalContext` for the agent's system prompt
- Hard guarantees enforced:
  - Token budget cap (default 1500 for SessionStart; configurable in `STOA.md`)
  - Relevance gate (skip injection if top hit cosine <0.65)
  - Top-of-prompt placement (never mid-conversation)
  - Provenance attached: every snippet carries `source_path` + `score`
- `stoa inject log [--session <id>] [--limit N]` — view injection history with full text
- Audit: every injection event appended to `.stoa/audit.log` with what was injected and which hook fired

**Exit criteria**:
- Injection observed in actual Claude Code session (golden-path validation, not just unit tests)
- Token cap enforced (test: index 10k pages, force a query that retrieves all, verify injection truncates)
- Relevance gate fires (test: deliberately irrelevant query → no injection emitted; verified in audit log)
- MINJA defense smoke test: a wiki page containing prompt-injection-style text ("Ignore prior instructions and ...") is wrapped and the agent does not act on it (manual test against Claude Code)
- `stoa inject log` returns full injection text with source paths
- Audit log is append-only and machine-readable

**Demo**: User opens a new Claude Code session in a Stoa workspace, sees `<stoa-memory>` block at top of system prompt with relevant entity pages; inspects via `stoa inject log`.

### M6 — v0.1 release

**Size**: M

**Deliverable**:
- Tag `v0.1.0`
- Release artifacts on GitHub: cross-compiled binaries for all 5 targets
- `cargo install stoa` works on fresh macOS + Linux machines
- `README.md` with: install, quickstart (5 commands to value), screenshot of injection in action
- Docs site (mkdocs at `docs.stoa.dev` or `kichelm.github.io/stoa`) covering: install, schema, capture, recall, injection, troubleshooting
- Demo video (90s) showing capture → query → injection loop
- Blog post: "Stoa v0.1: an open-core memory system for AI agents that doesn't trust the agent to remember"
- HN submission ("Show HN: Stoa")
- `CHANGELOG.md` documenting everything in v0.1
- Issue templates + contributing guide

**Exit criteria**:
- Fresh-machine install works (verified by a non-author on macOS + Linux)
- Quickstart in README produces a working workspace + first injection within 5 minutes
- LongMemEval recall@k linked from README
- Public ship

**Demo**: HN post live; user reports successful install + first injection.

---

## Cross-cutting tracks (always-on during MVP)

These run continuously across all milestones, not as discrete deliverables:

- **Performance budget**: Hook latency CI gate from M3 onward (<10ms p95). Any PR that regresses fails CI.
- **Benchmarks**: `benchmarks/longmemeval/` runner stays green from M4 onward. Every backend or recall change re-runs and re-publishes.
- **Adversarial testing**: MINJA defenses validated continuously from M5. New attack vectors land as test cases in `crates/stoa-recall/tests/minja/`.
- **Docs sync**: Every milestone updates `ARCHITECTURE.md` if mechanics change, `PRODUCT.md` if positioning changes, `CHANGELOG.md` always.
- **Honest benchmarking discipline**: No test-corpus changes without re-running prior backends and re-publishing. No headline numbers from a different corpus than the public one.

---

## Sequencing rules (load-bearing)

1. **No M after M0 starts until M0 ships.** The spike validates §15 architectural assumptions. If any fail, architecture revisions ship before any M1 code is written.
2. **No M3 merge without hook latency CI gate green.** The <10ms budget is the architecture's load-bearing claim; protecting it begins on day one of M3.
3. **No public release without published recall@k.** v0.1 ships with LongMemEval baseline numbers in `benchmarks/results/`. Without them, v0.1 stays unreleased.
4. **No injection feature without audit log.** M5 ships `stoa inject log` in the same release as the hook itself. Injection without inspection is not shippable.
5. **Demo before exit.** Every M's exit criteria includes a manual demo on a real Claude Code session, not just unit tests passing.

---

## What gets cut if MVP scope creeps

If the MVP timeline slips, cut in this order (least to most painful):

1. **Cross-platform binaries**: ship Linux + macOS only at v0.1; defer Windows to v0.1.1.
2. **Demo video**: launch with screenshots only; record video later.
3. **Embeddings opt-in default**: ship `stoa init --no-embeddings` as the default (BM25-only); opt-in to embeddings for power users.
4. **Heuristic redaction extensions**: ship the core regex set only (API keys, tokens, paths); defer locale-specific PII to v0.2.

What does **not** get cut, ever, even under deadline pressure:
- The <10ms hook latency budget (architecture assumption).
- MINJA-resistant XML wrapping on every injection (security).
- Always-flush capture (silent data loss is unrecoverable).
- Published recall@k against a public corpus (credibility).

---

## After MVP ships

See [ROADMAP-POST-MVP.md](./ROADMAP-POST-MVP.md) for the v0.2 → v1.0 plan.
