# Changelog

All notable changes to Stoa are documented here.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).
Until v1.0, the public CLI surface and on-disk layout may break between minor
releases; breaking changes are called out under `### Changed` and `### Removed`.

## [Unreleased]

### Changed — M-Pivot (2026-05-13): rebuilt around MemPalace

- **Stoa is now a Rust hook + curated LLM wiki layered over MemPalace** rather than a from-scratch retrieval substrate. Rationale: [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md).
- New Python daemon `stoa-recalld` hosts MemPalace in-process, owns the on-disk wiki, and exposes a 5-method JSON-RPC surface over a Unix domain socket. Rust hooks + CLI talk to it.
- `stoa-inject-hook` now handles both `SessionStart` AND `UserPromptSubmit`. Per-prompt wiki injection is the headline retrieval path; warm latency ~50–200 ms.
- `stoa-hook` posts a `mine` RPC to the daemon on `SessionEnd` / `Stop` rather than writing to a local queue.
- `stoa write` writes both the markdown file on disk AND a `kind=wiki`-tagged drawer in MemPalace via the daemon — wiki participates in MemPalace's hybrid BM25 + cosine retrieval.
- `stoa query` and `stoa-inject-hook` filter on `kind=wiki` by default; pass `--include-drawers` to query verbatim conversation drawers too.
- `stoa-harvest` and `stoa-crystallize` rewired to call the daemon. Default LLM is Anthropic (Claude Opus); workers no-op cleanly when `ANTHROPIC_API_KEY` is not set.
- `RecallBackend` trait survives, but the v0.1 impl is `MempalaceBackend` (Unix-socket client) and only `MempalaceBackend`.

### Removed — M-Pivot

- Rust crates: `stoa-queue`, `stoa-capture`, `stoa-bench`, `stoa-viz`, `stoa-render-mermaid`, `stoa-render-svg`, `stoa-render-tui`, `stoa-recall/backends/local-chroma-sqlite`.
- Python packages: `stoa-shared`, `stoa-embed`, `stoa-recall` (sidecar), `stoa-bench-judge`.
- Benchmark scaffolding under `benchmarks/`. MemPalace publishes its own LongMemEval / MemBench / LoCoMo numbers; Stoa cites upstream rather than re-running.
- The pre-pivot recall index (`.stoa/recall.db` + `.stoa/vectors/`). The new index lives at `.stoa/palace/` under MemPalace's directory layout.

### Notes

Pre-pivot crates published to crates.io at 0.1.0 are obsolete. The yank + republish plan is documented in [ROADMAP.md](./ROADMAP.md) §M-v0.1.

---

The history below records pre-pivot milestones for posterity. The code that backed
those milestones was removed in the pivot. The historical line still survives in git
under tags / earlier commits.

### Pre-pivot history

- **M0** — Validation spike (hook cold-start, `fastembed` parity, `cross` compilation).
- **M1** — Repo skeleton (Cargo workspace + Python `uv` workspace + Justfile + CI).
- **M2** — Wiki + CLI core (`stoa init` / `write` / `read` / `schema`).
- **M3** — Capture pipeline (queue-based hooks, regex PII redaction, capture worker).
- **M4** — Recall pipeline (in-house `LocalChromaSqliteBackend`, BM25 via SQLite FTS5).
- **M5** — SessionStart injection (`<stoa-memory>` envelope, MINJA defenses, audit log).
- **M6** — Release on-ramp (CHANGELOG, issue/PR templates, docs site, release-plz wiring).

[Unreleased]: https://github.com/marcoskichel/stoa/commits/main
