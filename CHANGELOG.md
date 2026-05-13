# Changelog

All notable changes to Stoa are documented here.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).
Until v1.0, the public CLI surface and on-disk layout may break between minor
releases; breaking changes are called out under `### Changed` and `### Removed`.

## [Unreleased]

The next entry below this line becomes `0.1.0` once the release is tagged.
Items shipped on `main` since the start of the project are recorded under
the milestone they belong to so the history matches `ROADMAP.md`.

### Added — M5 (SessionStart injection + MINJA defenses)

- `stoa-inject-hook` binary that handles Claude Code `SessionStart` events,
  resolves the workspace from `cwd`, builds a recall query from cwd
  basename / git remote / recently-edited wiki pages, and emits a
  `<stoa-memory>`-wrapped `additionalContext` block under a hard token cap.
- MINJA defense: the wrapper splices a U+2060 word joiner inside any
  `<stoa-memory` / `</stoa-memory` substring carried by snippet bodies,
  source paths, and queries so user content cannot escape the envelope.
- Stdin is bounded to 256 KiB; oversize payloads degrade to an empty
  injection rather than blocking session start.
- Audit log: every injection appends one JSONL line to `.stoa/audit.log`
  (`event: stoa.inject`, session id, query, hits, chars injected, full
  context). The log file is symlink-refused on open.
- `stoa inject log [--session <id>] [--limit N]` reads the audit log and
  prints injection history most-recent-first.

### Added — M4 (Recall pipeline)

- `stoa-recall` crate with the `RecallBackend` trait per ARCHITECTURE.md
  §6.1 and reciprocal rank fusion (k=60) across vector + BM25 + KG
  streams.
- `LocalChromaSqliteBackend` (default v0.1 backend) under
  `crates/stoa-recall/backends/local-chroma-sqlite/` — SQLite FTS5 for
  BM25 today; ChromaDB + KG tables wired for v0.2 swap-in.
- `stoa query <q> [--k N] [--json]` and `stoa index rebuild` subcommands.
- Benchmark scaffolding committed under `benchmarks/` for LongMemEval,
  MemoryAgentBench, MEMTRACK, BEAM, AgentLeak, and MTEB/BEIR; corpus
  download scripts under `benchmarks/corpus/`.

### Added — M3 (Capture pipeline)

- `stoa-hooks` static binary opening `.stoa/queue.db` (rusqlite, WAL,
  `synchronous=NORMAL`), inserting one row, and exiting under the
  <10 ms p95 cold-start budget.
- `stoa daemon` long-running process and capture worker draining the
  queue into redacted JSONL transcripts under `sessions/<id>.jsonl`.
- Regex PII redaction covering AWS / Stripe / OpenAI / Anthropic /
  GitHub PAT / bearer / JWT / configurable email + SSH/AWS/GPG paths.
- `stoa hook install --platform claude-code` registers the
  `Stop` / `SessionEnd` Claude Code hook.
- Append-only `.stoa/audit.log` for capture events.
- Lint-policy guard `PreToolUse` hook flagging edits to lint-affecting
  files for explicit permission.

### Added — M2 (Wiki + CLI core)

- `stoa init` scaffolds the workspace (`STOA.md`, `wiki/{entities,concepts,synthesis}/`,
  `raw/`, `sessions/`, `.stoa/`, `.gitignore`); idempotent.
- `stoa read <id>`, `stoa write <id> [--frontmatter file] [--body file]`,
  and `stoa schema [--check]` cover the manual wiki write side until
  harvest/crystallize land in v0.2.
- `serde_yaml` frontmatter parser validating against the `STOA.md`
  vocabulary; rejects unknown entity types, missing required fields,
  and invalid relationship types.
- Auto-generated `index.md` and `log.md`.
- E2E `trycmd` snapshot harness for the CLI.

### Added — M1 (Repo skeleton)

- Cargo workspace at root with every crate from ARCHITECTURE.md §16.2
  scaffolded (stub `lib.rs` + one passing test each).
- Python sidecar `uv` workspace with stub `stoa-shared`,
  `stoa-harvest`, `stoa-crystallize`, `stoa-embed` packages
  (transitional — slated for deletion at v0.3 when reimplemented in
  Rust).
- `Justfile` covering `build`, `test`, `lint`, `lint-docs`, `fmt`,
  `typecheck`, `file-length`, `deny`, `machete`, `release`, `ci`.
- GitHub Actions workflows: `rust.yml`, `python.yml`, `release.yml`,
  `codeql.yml`, `bench.yml`.
- Strict lint surface: clippy `pedantic` + `cargo` groups,
  `unsafe_code = forbid`, no `unwrap`/`expect`/`panic`/`todo`/`dbg!`
  in non-test code, `basedpyright` `strict`, ruff with ~40 rule
  families, hard caps of 25-line function bodies + 400-line files.
- `cargo-deny` policy banning `openssl` / `openssl-sys` /
  `native-tls` / `git2` / `libssh2-sys` (rustls-only).
- `crates/stoa-doclint` enforcing the doc-comment policy described in
  CLAUDE.md (six durable intent prefixes; `TODO:` is not allowed).

### Added — M0 (Validation spike)

- 1-page report at `benchmarks/spike-m0.md` validating the three
  load-bearing assumptions in ARCHITECTURE.md §15: hook cold-start
  <10 ms p95 (Linux + macOS), `fastembed` ONNX parity with the
  Python reference, and `cross` cross-compilation across all five
  v0.1 release targets.

### Documentation

- `PRODUCT.md` — positioning and value proposition.
- `ARCHITECTURE.md` — authoritative source of truth for the design.
- `ROADMAP.md` + `ROADMAP-POST-MVP.md` — milestone exit criteria.
- `README.md` — install + quickstart + screenshot of the value loop.
- `CONTRIBUTING.md`, `SECURITY.md`, `CODEOWNERS`.

### Security

- MINJA wrapper on every injection (M5) — non-negotiable per
  ROADMAP.md "What does not get cut, ever".
- `cargo-deny` advisory + license + bans gate on every PR.
- CodeQL workflow scanning Rust on every push.
- Dependabot for Cargo + GitHub Actions ecosystems.

[Unreleased]: https://github.com/marcoskichel/stoa/commits/main
