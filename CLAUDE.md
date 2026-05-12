# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project context

Stoa is an open-core knowledge + memory system for AI agents. The repo is **pre-v0.1**, currently at milestone **M1 (repo skeleton)** — every crate is a stub `lib.rs`/`main.rs` with one passing test, and concrete behavior lands in M2+ per [ROADMAP.md](./ROADMAP.md).

When in doubt about design, **[ARCHITECTURE.md](./ARCHITECTURE.md) is the authoritative source** (and is referenced from frontmatter / docstrings throughout the codebase). [PRODUCT.md](./PRODUCT.md) covers the why; ROADMAP files cover the order and exit criteria per milestone.

## Common commands

The `Justfile` is the canonical entrypoint — prefer `just <recipe>` over running `cargo`/`uv` directly so Rust + Python stay in sync.

```bash
just build          # cargo build --workspace --locked && uv sync --all-groups
just test           # cargo test + pytest
just lint           # clippy (-D warnings) + cargo fmt --check + ruff check + ruff format --check
just fmt            # cargo fmt + ruff format
just typecheck      # basedpyright (strict)
just file-length    # enforce 400-line file cap (Rust + Python)
just deny           # cargo-deny (licenses + advisories + bans)
just machete        # unused-dep finder
just ci             # full local gate (ci-rust + ci-python + deny)
just install-dev    # cargo install stoa-cli + uv sync
just release linux-x86_64   # cross-compile + tarball into dist/
```

Running a single test:

```bash
cargo test -p <crate> <test_name> -- --nocapture     # Rust, scoped to one crate
cd python && uv run pytest packages/<pkg>/tests/test_x.py::test_y -q   # Python
```

Note: `just bench` won't work yet — `stoa-bench` has no implementation until M5.

## Architecture: load-bearing invariants

Three persistent layers + four background workers (see [ARCHITECTURE.md §Overview](./ARCHITECTURE.md)):

1. **Layer 1 — Wiki** (`wiki/`, `raw/`, `sessions/`): plain markdown on disk, the canonical store. Survives if Stoa disappears.
2. **Layer 2 — Recall** (`.stoa/recall.db`, `.stoa/vectors/`): derived BM25 + embeddings + KG index, rebuildable from Layer 1.
3. **Layer 3 — CLI + hooks**: `stoa` CLI and `stoa-hook` binary; the agent-facing surface.

Two patterns are non-negotiable:

- **Layer 1 / Layer 2 split.** Nothing lives only in the derived index. `stoa rebuild` must always be able to regenerate `.stoa/` from `raw/` + `wiki/` + `sessions/`.
- **Hook → queue → worker.** Hooks must complete in **<10 ms p95** — they only insert into `.stoa/queue.db` (SQLite WAL) and return. All heavy work (redaction, embedding, harvest, crystallize) happens in async workers draining the queue. This is why `stoa-hooks` depends only on `stoa-core` + `stoa-queue`.

## Workspace layout

Cargo workspace (`Cargo.toml`, resolver 3, edition 2024, Rust 1.95 pinned via `rust-toolchain.toml`):

| Crate | Role |
|---|---|
| `stoa-core` | Schema, frontmatter, ids — concrete API lands in M2 |
| `stoa-cli` | `stoa` binary (clap-based) — orchestrates worker crates as subcommands |
| `stoa-hooks` | `stoa-hook` binary; <10 ms cold-start budget |
| `stoa-queue` | SQLite-backed work queue |
| `stoa-capture` | Capture worker + PII redaction |
| `stoa-recall` | `RecallBackend` trait + reciprocal rank fusion |
| `stoa-recall/backends/local-chroma-sqlite` | Default v0.1 backend (workspace member nested under `stoa-recall/backends/*`) |
| `stoa-viz` | Viz module + worker |
| `stoa-render-{mermaid,svg,tui}` | Render backends (resvg for SVG, ratatui+sixel for TUI) |
| `stoa-bench` | LongMemEval runner |

Python sidecar (`python/`, uv workspace, Python 3.13) — **transitional, deleted at v0.3** when harvest/crystallize/embed are reimplemented in Rust per [ARCHITECTURE.md §16.6](./ARCHITECTURE.md):

- `stoa-shared` — shared queue client
- `stoa-harvest`, `stoa-crystallize` — `instructor` + `anthropic`
- `stoa-embed` — `sentence-transformers`

`benchmarks/spike-m0/` is **deliberately excluded** from the Cargo workspace (see `Cargo.toml` `[workspace] exclude`). Don't add it back — it's the M0 validation spike, frozen on disk.

## Lint discipline

The lint configs are strict by design and CI runs `RUSTFLAGS="-D warnings"`. Read these before adding any `allow`/`expect`/`ignore`:

- **Workspace lints** (`Cargo.toml` `[workspace.lints]`): `unsafe_code = forbid`, `unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented`/`dbg_macro`/`exit` all deny in non-test code. Clippy `pedantic` + `cargo` groups on.
- **Hard caps** (not expressible via clippy alone):
  - Function body ≤ **25 lines** — clippy `too_many_lines` (threshold in `clippy.toml`) + `scripts/check_lengths.py` for Python.
  - File ≤ **400 lines** — `scripts/check-file-length.sh` (Rust) + `scripts/check_lengths.py` (Python). Override via `LIMIT=` env var if absolutely necessary.
- **Python** (`python/pyproject.toml`): basedpyright `typeCheckingMode = "strict"` with `reportAny = "error"` — `Any` is a hard error including via inference. Ruff selects ~40 rule families. `from __future__ import annotations` is required in every module (`isort.required-imports`).

**Escape hatches — always required, never bare:**

- Rust: `#[expect(<lint>, reason = "...")]`. Never `#[allow(...)]` (banned via `allow_attributes` + `allow_attributes_without_reason`).
- Python: `# type: ignore[<rule>]` with explicit rule code. Never bare `# noqa` or `# type: ignore`.

If you need to relax a cap, the right move is usually to **split the function/file**, not to add an escape hatch.

## Supply chain (cargo-deny)

`deny.toml` bans `openssl`, `openssl-sys`, `native-tls`, `git2`, `libssh2-sys` — Stoa is rustls-only by policy. If you reach for a crate that pulls these in, find a rustls-equivalent or `default-features = false` it out (see how `reqwest` is configured in `Cargo.toml`).

## Local env traps (Arch / Linux)

Documented during M0 spike — see [CONTRIBUTING.md](./CONTRIBUTING.md) for the full version:

1. **System Rust shadows rustup on Arch.** `/usr/bin/cargo` (pacman) takes precedence over `~/.cargo/bin/cargo`. `cross` will fail with `invalid toolchain name: 'usr'`. Fix locally:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   export RUSTUP_TOOLCHAIN=stable
   ```
2. **`cross` wine seccomp on Linux 7.x.** Windows cross-build needs `CROSS_CONTAINER_OPTS="--security-opt seccomp=unconfined"`. Production CI uses native runners, so this only matters locally.

## CI

Three workflows in `.github/workflows/`:

- `rust.yml` — fmt + clippy + build + test on Linux/macOS/Windows; cargo-deny + cargo-machete on Linux.
- `python.yml` — ruff + basedpyright + length caps + pytest on Linux/macOS.
- `release.yml` — on tag push, builds tarballs across 5 targets (linux x86_64/aarch64, macos x86_64/aarch64, windows x86_64) via runner matrix.

`just ci` runs the local equivalent of the gates that matter.

## Commits and PRs

Use [Conventional Commits](https://www.conventionalcommits.org/) for **both commit messages and PR titles**: `<type>(<scope>)?: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Breaking changes: append `!` (e.g. `feat!: ...`) and include a `BREAKING CHANGE:` footer.
- Scope is optional but encouraged when work is crate- or milestone-bound: `feat(stoa-cli): ...`, `chore(M1): ...`.
- Subject: imperative mood, no trailing period, ≤72 chars.
- See `git log` for prevailing form (e.g. `M1: repo skeleton (...)`, `docs: add ...`) — migrate to strict Conventional Commits going forward.

Never `--no-verify`. Don't commit `target/`, `python/.venv/`, or `dist/` (all gitignored).
