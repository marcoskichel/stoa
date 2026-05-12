# Roadmap — Post-MVP (v0.2 → v1.0)

This is the shipping plan after Stoa v0.1 ships. For the MVP plan, see [ROADMAP.md](./ROADMAP.md).

The implementation tier list lives in [ARCHITECTURE.md §14](./ARCHITECTURE.md). This document is the *order* and *exit criteria* — what gets shipped when, and how we know it's done.

Sizes are t-shirt: **S** = days, **M** = 1–2 weeks, **L** = 3–4 weeks, **XL** = >1 month. Estimates assume one focused contributor; parallelize where dependencies allow.

Phase ordering is firm; milestone ordering within a phase is flexible based on what unblocks what.

---

## Phase 2 — Distillation → v0.2

The MVP delivers passive capture + retrieval + injection. Phase 2 adds **active distillation**: the wiki starts writing itself from session transcripts via harvest (per-session, fine-grained) and crystallize (cross-session, with invalidation).

### M7 — Harvest worker + quality gating

**Size**: L

**Deliverable**:
- Python sidecar `stoa-harvest` package: `instructor` + Anthropic SDK
- JSON-schema-validated extraction of entities, decisions, relationships, observations from one session JSONL
- Quality threshold (`quality ≥ 3` default, configurable in `STOA.md`)
- Idempotent writes to `wiki/entities/` keyed by `(source_id, harvest_version)`
- `harvested_from` provenance trail in entity frontmatter
- Conflict surface: harvest writes flagged conflicts to `lint-report.md` (M9 consumes this)
- `stoa harvest <session-id|--all-pending>` manual trigger

**Exit criteria**:
- Format-error rate <5% on a benchmark transcript suite
- Quality-gated drops noise (low-importance tool calls, throwaway exploration) — measured against a labeled fixture set
- Re-running harvest on same session updates same pages (no duplicates)
- Throughput: 30s end-to-end on a 100-turn session

**Demo**: End a Claude Code session, see entity pages updated within 30s with new relationships and observations from the conversation.

### M8 — Crystallize + invalidation pass

**Size**: L

**Deliverable**:
- Nightly cron (`crystallize.tick`) running candidate scan
- Promotion criteria from [ARCHITECTURE.md §9.2](./ARCHITECTURE.md): minimum length, decision marker, ≥2 entities, no overlapping synthesis exists
- Synthesis drafts written to `wiki/synthesis/<slug>.draft.md` with `status: draft`
- Invalidation pass: LLM identifies what existing claims the new session contradicts → supersession proposals
- Auto-applied supersession above confidence threshold; below-threshold proposals to `lint-report.md`
- `stoa crystallize [--dry-run]` manual trigger
- `stoa crystallize --status` shows last run + next scheduled

**Exit criteria**:
- 7-day continuous capture run produces ≥1 synthesis draft per relevant thread + ≥1 supersession proposal
- Drafts wait for human review by default (no auto-publish in v0.2)
- Invalidation pass catches a deliberately planted stale claim in a fixture corpus
- Idempotent: re-running crystallize on same sessions doesn't produce duplicate drafts

**Demo**: Review draft synthesis page after a week of usage; see one auto-applied supersession in `log.md`.

### M9 — Lint

**Size**: M

**Deliverable**:
- `stoa lint [--fix det] [--report]`
- Deterministic auto-fix: broken links, frontmatter violations, missing required fields, dangling supersedes refs, mtime drift
- Heuristic report-only: contradictions, stale claims, missing entities, schema violations, page quality below bar, likely duplicates
- Viz anti-pattern lint integrated (per [ARCHITECTURE.md §12.4](./ARCHITECTURE.md))
- `lint-report.md` written with categorized findings

**Exit criteria**:
- Auto-fix doesn't false-positive on a representative test corpus (zero silent corruption)
- Heuristic report surfaces real contradictions (validated on a planted-contradiction fixture)
- Viz anti-pattern lint rejects 3D charts, rainbow palettes, pies >5 slices, etc.

**Demo**: Run `stoa lint --report` on a long-lived workspace; see categorized issues with file:line references.

### M10 — UserPromptSubmit + PreCompact injection

**Size**: M

**Deliverable**:
- UserPromptSubmit handler: per-prompt retrieval with sliding similarity threshold
- PreCompact handler: `systemMessage`-only mode (never `block` — avoids mempalace #856/858/906/941/955 bug class)
- Hard guarantees from M5 (token cap, relevance gate, MINJA wrapper, audit log) extended to both new hooks
- Per-hook token caps: 500 for UserPromptSubmit, 1000 for PreCompact
- Sliding similarity gate: threshold tunable per workspace

**Exit criteria**:
- Per-turn injection working without runaway token cost (measured: average tokens per session)
- PreCompact rescues entity mentions across compaction boundary (validated in long Claude Code session)
- No infinite re-fire (smoke test: PreCompact handler returns systemMessage for 100 consecutive compactions without loop)
- Injection utilization ratio measured: % of injected tokens that the agent actually references in the next response

**Demo**: Long Claude Code session crosses compaction boundary; entities mentioned pre-compaction are surfaced post-compaction via PreCompact injection.

### M11 — Terminal viz + Mermaid embeds

**Size**: L

**Deliverable**:
- `stoa render <id> [--view <name>] [--backend mermaid|ratatui|sixel] [--out path]`
- ratatui backend: sparklines, bars, ranked lists with inline bar glyphs
- Mermaid backend: entity neighborhoods (flowchart), log timelines (timeline), distillation flow diagrams
- Sixel backend (capability-detected): SVG → bitmap → terminal pixels for capable terminals (WezTerm, iTerm2 3.4+, kitty, foot)
- Viz spec data model (`VizSpec`) per [ARCHITECTURE.md §12.4](./ARCHITECTURE.md)
- Anti-pattern lint integrated with M9 lint pass
- Default views per page kind from §12.3 mapping table

**Exit criteria**:
- `stoa render ent-redis --backend mermaid` produces valid Mermaid graph syntax that renders correctly on GitHub + Obsidian
- `stoa render ent-redis --backend ratatui` shows interactive terminal view
- Sixel detection works on supported terminals; falls back to ratatui ASCII on plain xterm
- Viz anti-pattern lint catches every banned pattern from §12.2

**Demo**: `stoa render ent-redis --backend mermaid > out.md` and view in GitHub; same entity rendered as ratatui in terminal.

### M12 — v0.2 release

**Size**: M

**Deliverable**:
- Tag `v0.2.0`
- Changelog
- Migration notes from v0.1 (no breaking changes expected)
- Blog post: "Stoa v0.2: the wiki writes itself now"
- Updated docs site

**Exit criteria**:
- `cargo install stoa` upgrades cleanly from v0.1
- All M7–M11 features documented in docs site

---

## Phase 3 — Cross-platform + Rust migration → v0.3

Phase 3 broadens platform support (Cursor, Codex), shrinks the Python sidecar by migrating the embedding worker to Rust, adds pre-rendered SVG snapshots to the viz worker, and ships the optional MCP wrapper.

### M13 — Cursor + Codex hooks

**Size**: M

**Deliverable**:
- Per-platform hook scripts under `crates/stoa-hooks/`
- `stoa hook install --platform cursor` and `--platform codex`
- Capture parity with Claude Code: same redaction, same JSONL format, same audit trail
- Hook routing when same workspace is touched by multiple agents (single workspace can hold multi-platform sessions)

**Exit criteria**:
- Sessions from Claude Code, Cursor, and Codex all captured into the same workspace with consistent JSONL format
- Hook latency CI gate green for all three platforms

**Demo**: Use Cursor and Codex on the same project; see unified harvest output combining all sessions.

### M14 — Lifecycle workflow

**Size**: M

**Deliverable**:
- `stoa supersede <old-id> <new-id>` — explicit supersession with `supersedes` frontmatter linkage
- Staleness pass: nightly job flagging pages whose source citations are older than freshness window (default 180 days, per-kind in `STOA.md`)
- Derived relationship confidence: computed from source count, source recency, and contradiction signals
- `stoa freshness [--report]` — print/report on staleness across workspace

**Exit criteria**:
- Supersession round-trips: old version preserved, marked stale, excluded from default recall (opt-in via `--include-superseded`)
- Staleness pass is non-destructive (flag-only, never deletes)
- Derived confidence numbers are reproducible from source state (not gut-set)

**Demo**: Supersede an outdated entity, see new version surfaced in recall while old version stays accessible via `--include-superseded`.

### M15 — Embedding worker → Rust (the v0.2/v0.3 trigger event)

**Size**: L

**Deliverable**:
- Spike: benchmark `fastembed` + `ort` vs `tract` for bge-small-en-v1.5 on all 5 release targets
- Cross-compile pipeline validated for the embedding-enabled binary
- Pick winner based on data (throughput vs cross-compile portability vs binary size)
- Migrate embed worker from Python `sentence-transformers` to Rust
- Python sidecar shrinks: only `stoa-harvest` and `stoa-crystallize` remain
- `cargo install stoa` ships with embedding included

**Exit criteria**:
- Embedding throughput documented (sentences/sec on CPU, M-series Mac, Linux x86_64)
- Cross-compile CI green for all 5 targets with embedding included
- Recall@k parity: LongMemEval results within 1% of Python `sentence-transformers` baseline
- `python/packages/stoa-embed/` deleted; uv lockfile shrinks

**Demo**: Fresh-machine `cargo install stoa` on Windows produces a working binary with embedding (no Python sentence-transformers needed for embedding path).

### M16 — Pre-rendered SVG snapshots

**Size**: M

**Deliverable**:
- Viz worker (4th background worker, joining capture/harvest/crystallize)
- Subscribes to `wiki.page.written`, `wiki.page.deleted`, `crystallize.tick`, explicit `stoa render` invocations
- SVG generation via `resvg` + Sigma snapshot (headless)
- Outputs to `.stoa/renders/<page-id>-<view>.<ext>` keyed by content hash
- Optional snapshot to `wiki/.renders/` (toggle in `STOA.md`) for git-portable embed
- `stoa render --bake` — pre-render every page's default views

**Exit criteria**:
- Every wiki page write triggers SVG generation within 5s
- Snapshots embeddable in standard markdown renderers (GitHub, Obsidian, mkdocs)
- Content-hash invalidation works: unchanged pages skip re-render
- Render failures are non-blocking (audit-logged, wiki page still readable)

**Demo**: Push wiki to GitHub, see entity neighborhood SVG render in browser without any GitHub plugin installed.

### M17 — MCP wrapper

**Size**: M

**Deliverable**:
- Thin MCP server (`crates/stoa-mcp/`) shelling out to CLI
- Tool inventory: `stoa_query`, `stoa_read`, `stoa_inject_log`, `stoa_note`
- Schema declarations for each tool
- Install via MCP-aware client config (one config block per client)

**Exit criteria**:
- Claude Code with MCP enabled shows stoa tools in panel
- Each tool call shells out to existing CLI command and returns structured response
- No new code paths beyond the CLI surface (the MCP wrapper is a translation layer only)

**Demo**: Claude Code MCP panel shows `stoa_query`; tool call equivalent to `stoa query`.

### M18 — MempalaceBackend (conditional)

**Size**: M

**Trigger condition**: Mempalace API has been stable for ≥60 days (no breaking releases). If condition not met, this milestone is skipped and the slot is taken by another v0.3 work.

**Deliverable**:
- `MempalaceBackend` adapter implementing `RecallBackend` trait
- Published recall@k against the same test corpus as `LocalChromaSqliteBackend`
- Documentation on when to pick which backend

**Exit criteria**:
- `quality_suite` method passes on `MempalaceBackend`
- Backend swap quality-gated (no silent recall regression)
- Recall@k numbers published in `benchmarks/results/`

**Demo**: Toggle backend in config, recall comparable on same query set.

### M19 — v0.3 release

**Size**: M

**Deliverable**:
- Tag `v0.3.0`
- Changelog covering M13–M18
- Migration notes (Python sidecar shrinks; downstream impact for users with custom hook scripts)
- Blog post: "Stoa v0.3: cross-platform capture, the embedding worker is now Rust"

**Exit criteria**:
- `cargo install stoa` upgrades cleanly from v0.2
- All M13–M18 features documented

---

## Phase 4 — All-Rust + multi-agent + web → v0.4

Phase 4 completes the Rust migration (Shape B from [ARCHITECTURE.md §15](./ARCHITECTURE.md)), adds multi-agent support with scoping and mesh sync, and ships the web UI.

### M20 — Harvest + crystallize → Rust

**Size**: XL

**Trigger condition**: `rstructor` has been stable (no breaking releases) for ≥60 days, OR an official Anthropic Rust SDK has shipped.

**Deliverable**:
- Migrate `stoa-harvest` and `stoa-crystallize` from Python to Rust
- Use `rstructor` (or successor) + Anthropic Rust SDK + `async-openai`
- Wrap LLM client behind internal trait so SDK swaps don't ripple into worker logic
- `python/` directory deleted entirely
- Single-binary `cargo install stoa` with no Python runtime needed anywhere

**Exit criteria**:
- Format-error rate parity with Python `instructor` baseline (within 2pp on benchmark suite)
- Quality gating validated against Phase 2 baseline
- Throughput: harvest end-to-end ≤45s on a 100-turn session (allows for higher Rust ergonomics overhead)
- LongMemEval recall@k parity (within 1%)

**Demo**: All-Rust install on fresh machine; no Python install anywhere; harvest + crystallize working end-to-end.

### M21 — LanceDB adapter (conditional)

**Size**: L

**Trigger condition**: LanceDB Rust FTS API stable (no breaking changes for ≥60 days post Tantivy → native FTS migration).

**Deliverable**:
- `LanceDbBackend` adapter implementing `RecallBackend` trait
- Replaces ChromaDB + sentence-transformers in the default backend stack (using `fastembed` from M15 for embeddings)
- Published recall@k matches `LocalChromaSqliteBackend` baseline

**Exit criteria**:
- Backend swap quality-gated
- Storage format portability: data accessible via DuckDB / Polars (not a black box)
- Migration path from `LocalChromaSqliteBackend` documented

**Demo**: Toggle default backend; recall comparable; data inspectable via DuckDB.

### M22 — Multi-agent: scoping + promotion

**Size**: L

**Deliverable**:
- Per-agent scope config in `STOA.md`: read-all default, write to own `wiki/agents/<agent-id>/` subdirectory
- Promotion workflow: agent's private observation → shared wiki content via crystallization with explicit `promote` operation in audit log
- `stoa scope` command for inspecting per-agent scopes
- `stoa promote <agent-page-id>` for manual promotion

**Exit criteria**:
- 2 agents on same workspace produce non-conflicting writes (no clobbering, no race conditions on entity pages)
- Promotion workflow surfaces in lint as "candidates for promotion"
- Audit log distinguishes per-agent operations

**Demo**: Two Claude Code sessions on same workspace; promote shared synthesis from one agent's private space to shared wiki.

### M23 — Multi-agent: mesh sync

**Size**: L

**Deliverable**:
- rsync or git-based workspace sync between machines
- Conflict resolution: last-write-wins on observations, three-way merge with human review on synthesis
- Embedding indexes are agent-local (rebuilt on sync, not synced directly)
- `stoa sync [--mode rsync|git] <peer>` command

**Exit criteria**:
- 2 machines converge on shared state after sync
- Conflicting writes surface in lint, not silently dropped
- Sync round-trip <60s for a 1000-page workspace

**Demo**: Edit on machine A, run sync, see on machine B; deliberately conflict and resolve.

### M24 — Web UI

**Size**: XL

**Deliverable**:
- `stoa serve [--port 7000]` — local web server bound to 127.0.0.1
- TypeScript + Vite + (optional) React frontend
- Sigma.js + Graphology for entity graph (force-directed, degree-filtered, matrix toggle for dense neighborhoods)
- Observable Plot for statistical charts (memory growth, distillation quality, activity heatmap)
- visx primitives for LineUp ranked-hit display + UpSet plots
- `web/` directory under repo root with bun/pnpm workspace
- Markdown page renderer with typographic hierarchy + inline entity highlighting + 70-char line width

**Exit criteria**:
- Web UI shows entity neighborhood, growth charts, distillation reports, ranked search
- All viz from §12.3 data-type mapping table renders correctly
- Anti-patterns rejected at render time (consistent with terminal viz from M11)
- Browser opens at workspace overview within 2s of `stoa view`

**Demo**: `stoa view` opens browser at workspace overview; explore entity graph, search, view synthesis pages.

### M25 — v0.4 release

**Size**: M

**Deliverable**:
- Tag `v0.4.0`
- Changelog covering M20–M24
- Migration notes (Python sidecar gone; multi-agent enabled; web UI optional)
- Blog post: "Stoa v0.4: all-Rust, multi-agent, web UI"

**Exit criteria**:
- All M20–M24 features documented

---

## Phase 5 — Production → v1.0

Phase 5 hardens the system for production workloads, ships encryption-at-rest, and decides on the paid layer.

### M26 — Hardening

**Size**: L

**Deliverable**:
- Fuzz testing on capture/redaction (`cargo-fuzz`)
- Stress test workers with adversarial inputs (oversized JSONL, malformed frontmatter, concurrent writes)
- Audit log query CLI (`stoa audit [--filter ...]`)
- Performance regression suite

**Exit criteria**:
- 30-day continuous run: zero data-correctness issues
- Audit query returns in <100ms on 1M-event log
- Fuzz coverage report committed
- No `unsafe` blocks added without justification

**Demo**: Bug bounty program announcement; reported issues triaged within 48h.

### M27 — Encryption at rest

**Size**: L

**Deliverable**:
- age/sops or per-workspace AES encryption for `sessions/`
- Key recovery story documented (mnemonic backup, hardware token support)
- Opt-in via `STOA.md` config

**Exit criteria**:
- Reversible round-trip on all session data
- Key loss recoverable from documented procedure
- Encrypted sessions still searchable (BM25 + vector indexes built post-decryption in worker)

**Demo**: Encrypt workspace; verify sessions unreadable without key; decrypt and verify recall works.

### M28 — Paid layer evaluation (ongoing)

**Size**: ongoing

**Deliverable**: Decision document on whether to ship paid layer (sync, team, hosted, audit) or stay pure OSS.

**Inputs to decision**:
- OSS adoption metrics: stars, monthly active installs, retention
- Community demand signal: paid-layer feature requests, sustaining-membership inquiries
- Maintenance burden signal: contributor count, issue throughput
- Reference: Obsidian's path (free local + paid sync became viable at ~10k MAU)

**Exit criteria**:
- Decision committed (even if "not yet"): a `docs/paid-layer-decision.md` with the data and the reasoning

### M29 — v1.0 release

**Size**: M

**Trigger conditions**: M26 + M27 done; M28 decision recorded.

**Deliverable**:
- Tag `v1.0.0`
- Production support commitments (security patch SLA, semver guarantees)
- Paid layer launch (if M28 says go) or doubled-down OSS commitment (if not)
- Changelog covering Phase 5
- Blog post: "Stoa 1.0"

**Exit criteria**:
- All M26 + M27 features documented
- Public ship

---

## Cross-cutting tracks (always-on, post-MVP)

These extend the MVP cross-cutting tracks ([ROADMAP.md](./ROADMAP.md)):

- **Backend quality**: Every `RecallBackend` adapter (Mempalace, LanceDB, future) publishes recall@k against same corpus before merge.
- **MINJA evolution**: New attack vectors land as test cases. Defense effectiveness re-validated each release.
- **Migration safety**: Every minor release tested for upgrade path from prior minor (no silent data loss on upgrade).
- **Contributor docs**: Issue triage, PR review SLAs, governance model documented as community grows.

---

## Conditional milestones — explicit go/no-go

Three milestones are conditional on external state. If conditions aren't met, the slot stays open for parallel work or schedule contraction.

| Milestone | Condition | Fallback if condition fails |
|---|---|---|
| **M18 MempalaceBackend** | Mempalace API stable ≥60 days | Skip; revisit at end of Phase 3 |
| **M20 harvest+crystallize → Rust** | `rstructor` stable ≥60 days OR official Anthropic Rust SDK ships | Defer to Phase 5; keep Python sidecar through v0.4 |
| **M21 LanceDB adapter** | LanceDB Rust FTS API stable ≥60 days post Tantivy migration | Skip; ChromaDB stays default |

These conditions are not scope cuts — they're risk gates. Shipping a fragile adapter is worse than not shipping it.

---

## When to revisit this roadmap

Revisit conditions:
- Quarterly: review which milestones got faster/slower than t-shirt-sized; recalibrate.
- Phase boundary: before starting Phase N+1, confirm Phase N's exit criteria all met.
- Major dependency change: if Anthropic API, Claude Code hooks, or Rust ecosystem shifts meaningfully, audit affected milestones.
- User feedback: if v0.1/v0.2 reveals a wrong assumption about user need, the post-MVP plan adjusts before the next release.

The roadmap is not a contract; it's a current best plan. Reality wins.
