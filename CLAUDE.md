# CLAUDE.md

Guidance for Claude Code working in this repository.

## Project context

Stoa is a **Rust hook + curated LLM wiki layered over [MemPalace](https://github.com/MemPalace/mempalace)**. The pivot from a from-scratch retrieval substrate to a MemPalace-backed daemon landed on 2026-05-13.

When in doubt about design, **[ARCHITECTURE.md](./ARCHITECTURE.md) is the authoritative source**. [PRODUCT.md](./PRODUCT.md) covers positioning; [ROADMAP.md](./ROADMAP.md) covers order of work; [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md) records the pivot.

## Common commands

```bash
just build          # cargo build --workspace --locked && uv sync --all-groups
just test           # cargo test + pytest
just lint           # clippy (-D warnings) + cargo fmt --check + ruff check + ruff format --check
just fmt            # cargo fmt + ruff format
just typecheck      # basedpyright
just file-length    # enforce 400-line file cap (Rust + Python)
just deny           # cargo-deny (licenses + advisories + bans)
just machete        # unused-dep finder
just ci             # full local gate (ci-rust + ci-python + deny)
just install-dev    # cargo install stoa-cli + uv sync
just release linux-x86_64   # cross-compile + tarball into dist/
just docs           # mkdocs build --strict
```

Running a single test:

```bash
cargo test -p <crate> <test_name> -- --nocapture
cd python && uv run pytest packages/<pkg>/tests/test_x.py -q
```

## Architecture: load-bearing invariants

Three surfaces (see [ARCHITECTURE.md](./ARCHITECTURE.md)):

1. **Stoa surface (Rust)** — `stoa`, `stoa-hook`, `stoa-inject-hook`. Hooks must complete in <10 ms p95 (`stoa-hook`) or <500 ms warm (`stoa-inject-hook`).
2. **`stoa-recalld` (Python)** — long-lived daemon. Hosts MemPalace in-process, owns the on-disk wiki, exposes a 5-method JSON-RPC over `$XDG_RUNTIME_DIR/stoa-recalld.sock`.
3. **MemPalace** — pluggable retrieval substrate. v0.1 ships only the MemPalace adapter; the `RecallBackend` trait keeps the seam open.

Two patterns are non-negotiable:

- **Wiki on disk is canonical.** Every wiki page is markdown under `wiki/`; `.stoa/palace/` is derived. `stoa write` writes both. Hand-edits survive in git, but the index will not see them until `stoa-harvest run` or until the page is re-written.
- **Hook → daemon RPC.** Rust hooks shoot one JSON line at the daemon and exit. All heavy work (BM25/cosine, KG, LLM distillation) lives in the daemon or in the Python workers it dispatches. If the daemon is down, hooks still exit 0 so a missing daemon never breaks the agent loop.

## Workspace layout

Cargo workspace (`Cargo.toml`, resolver 3, edition 2024, Rust 1.95):

| Crate | Role |
|---|---|
| `stoa-core` | Wiki schema, frontmatter, IDs |
| `stoa-recall` | `RecallBackend` trait + `MempalaceBackend` (Unix-socket client) |
| `stoa-cli` | `stoa` binary (clap-based) — workspace + wiki + daemon orchestration |
| `stoa-hooks` | `stoa-hook` binary (SessionEnd → `mine` RPC, <10 ms budget) |
| `stoa-inject-hooks` | `stoa-inject-hook` binary (SessionStart + UserPromptSubmit → `search` RPC + MINJA envelope) |
| `stoa-doclint` | Doc-comment policy linter |

Python (`python/`, uv workspace, Python 3.13):

- `stoa-recalld` — the daemon; hosts MemPalace, serves the JSON-RPC socket.
- `stoa-harvest` — one-shot LLM worker; distills MemPalace drawers into `wiki/{entities,concepts}/*.md`.
- `stoa-crystallize` — one-shot LLM worker; produces `wiki/synthesis/*.md` pages.

## Lint discipline

The lint configs are strict by design and CI runs `RUSTFLAGS="-D warnings"`. Read these before adding any escape hatch:

- **Workspace lints** (`Cargo.toml`): `unsafe_code = forbid`, `unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented`/`dbg_macro`/`exit` deny in non-test code. Clippy `pedantic` + `cargo` groups on.
- **Hard caps**:
  - Function body ≤ **25 lines** — clippy `too_many_lines` (threshold in `clippy.toml`) + `scripts/check_lengths.py` for Python.
  - File ≤ **400 lines** — `scripts/check-file-length.sh` (Rust) + `scripts/check_lengths.py` (Python).
- **Python** (`python/pyproject.toml`): basedpyright `strict` with `reportAny = "error"`. Per-package overrides relax this where MemPalace's typed surface is `Any`-heavy.
- **Comment policy** (`crates/stoa-doclint`, run via `just lint-docs`):
  - Doc comments `///`, `//!`, `/** */`, `/*! */` are always allowed.
  - Bare `//` inline notes are forbidden unless they open with `SAFETY:`, `FIXME:`, `HACK:`, `PERF:`, `NOTE:`, or `WHY:`. `TODO:` is intentionally not allowed — track TODOs in the issue tracker.

**Escape hatches — always required, never bare:**

- Rust: `#[expect(<lint>, reason = "...")]`. Never `#[allow(...)]`.
- Python: `# type: ignore[<rule>]` with explicit rule code.

## Supply chain (cargo-deny)

`deny.toml` bans `openssl`, `openssl-sys`, `native-tls`, `git2`, `libssh2-sys` — Stoa is rustls-only by policy.

## CI

Workflows in `.github/workflows/`:

- `rust.yml` — fmt + clippy + build + test on Linux/macOS/Windows; cargo-deny + cargo-machete on Linux.
- `python.yml` — ruff + basedpyright + length caps + pytest on Linux/macOS.
- `release.yml` — on tag push, builds tarballs across 5 targets.
- `release-plz.yml` — automation for crates.io + GitHub Release flow.

`just ci` runs the local equivalent.

## Commits and PRs

[Conventional Commits](https://www.conventionalcommits.org/) for both commit messages and PR titles: `<type>(<scope>)?: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Subject: imperative mood, no trailing period, ≤72 chars.

Never `--no-verify`. Don't commit `target/`, `python/.venv/`, or `dist/`.
