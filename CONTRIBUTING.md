# Contributing to Stoa

## Quick start

Fresh clone (assumes `rustup` + `uv` already installed):

```bash
./scripts/bootstrap.sh        # installs dev tools + builds both workspaces
just ci                       # full local gate (lint + test + supply chain)
```

Day-to-day:

```bash
just install-dev              # cargo install stoa-cli + uv sync sidecar
just dev                      # interactive Rust dev loop (bacon)
just watch                    # cross-language lint+test on save
just docs                     # build the docs site locally
```

## Layout

See [ARCHITECTURE.md](./ARCHITECTURE.md). Six Rust crates under `crates/` (`stoa-core`, `stoa-recall`, `stoa-cli`, `stoa-hooks`, `stoa-inject-hooks`, `stoa-doclint`). Three Python packages under `python/packages/` (`stoa-recalld`, `stoa-harvest`, `stoa-crystallize`). The pivot on 2026-05-13 reshaped both — see [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md).

## Toolchain

- [`rustup`](https://rustup.rs) — toolchain manager (pinned 1.95 via `rust-toolchain.toml`).
- [`uv`](https://docs.astral.sh/uv/) — Python env manager (Python 3.13).
- [`just`](https://just.systems) — task runner.
- [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) — license + advisory + sources gate.
- [`cargo-machete`](https://github.com/bnjbvr/cargo-machete) — unused-dep finder.
- [`cross`](https://github.com/cross-rs/cross) — non-native release builds.
- [`bacon`](https://dystroy.org/bacon/) — interactive Rust dev loop (`just dev`).
- [`watchexec`](https://watchexec.github.io/) — cross-language file watcher (`just watch`).

You ALSO need MemPalace installed for any end-to-end testing:

```bash
uv tool install mempalace
```

## Coding rules

The lint configs are strict by design. See `Cargo.toml` workspace lints, `clippy.toml`, `rustfmt.toml` (Rust); `python/pyproject.toml` `[tool.ruff]` and `[tool.basedpyright]` (Python).

Highlights:

- **Function body ≤ 25 lines**, **file ≤ 400 lines**. Enforced by `clippy::too_many_lines` + `scripts/check-file-length.sh` (Rust) and Pylint `max-statements` + `scripts/check_lengths.py` (Python).
- **Rust**: `unsafe_code = forbid` at workspace level. Override per-crate with a `reason = "..."`. No `unwrap`/`expect`/`panic`/`todo`/`dbg!` in non-test code.
- **Python**: basedpyright strict at the workspace root with per-package overrides where MemPalace's typed surface is `Any`-heavy.
- **Escape hatches**: `#[expect(<lint>, reason = "...")]` in Rust and `# type: ignore[<rule>]` with explicit rule code in Python. Never bare `#[allow]` or `# noqa`.
- Imports: `from __future__ import annotations` is required in every Python module.
- **Comment policy** (`just lint-docs`, runs `stoa-doclint`). Doc comments are always allowed; bare `//` inline notes must open with one of `SAFETY:`, `FIXME:`, `HACK:`, `PERF:`, `NOTE:`, `WHY:`. `TODO:` is intentionally not allowed.
- **Doc-comment content.** Describe how the thing works — invariants, edge cases, error conditions, non-obvious constraints. Not a place for milestone or roadmap pointers, TODO lists, implementation history, authorship, or dates.

## Environment traps (Arch / Linux)

These bit the M0 spike author once; documented so contributors hit them at most once.

1. **System Rust vs rustup on Arch Linux.** Arch's `/usr/bin/cargo` shadows `~/.cargo/bin/cargo` in `PATH`. `cross-rs` reads rustup metadata to resolve the active toolchain; with system Rust active it fails with `invalid toolchain name: 'usr'`. Fix:

   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   export RUSTUP_TOOLCHAIN=stable
   ```

   CI uses rustup-managed toolchains, so this is local-only.

2. **Wine in cross-rs windows-gnu image on Linux ≥ 7.x.** `wineboot` fails inside the container with `wine: socket : Function not implemented` under the default Docker seccomp profile. Fix:

   ```bash
   export CROSS_CONTAINER_OPTS="--security-opt seccomp=unconfined"
   ```

   Production CI builds Windows natively on a `windows-latest` runner.

## CI

- `.github/workflows/rust.yml` — fmt + clippy + build + test on Linux/macOS/Windows; cargo-deny + cargo-machete on Linux.
- `.github/workflows/python.yml` — ruff lint+format check + basedpyright + length caps + pytest on Linux/macOS.
- `.github/workflows/release.yml` — runs on tag push; builds release tarballs across 5 targets via a runner matrix.
- `.github/workflows/release-plz.yml` — opens release PRs on push to main; cuts the tag + publishes on release-PR merge.

## Commits and PR titles

Use [Conventional Commits 1.0.0](https://www.conventionalcommits.org/) for **both commit messages and PR titles**: `<type>(<scope>)?: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Breaking changes append `!` (e.g. `feat!: ...`) and include a `BREAKING CHANGE:` footer.
- Scope is optional but encouraged when work is crate- or milestone-bound: `feat(stoa-cli): ...`, `chore(M-Pivot): ...`.
- Subject: imperative mood, no trailing period, ≤72 chars.

Never commit secrets. Never commit `target/`, `.venv/`, or `dist/` (covered by `.gitignore`).

## Changelog

`CHANGELOG.md` follows [keep-a-changelog 1.1.0](https://keepachangelog.com/en/1.1.0/):

- Keep an `## [Unreleased]` section at the top of the file *at all times*. After tagging, do not rename `[Unreleased]` — add a new `## [<version>] - <YYYY-MM-DD>` section below it.
- `scripts/check-changelog.sh` (run by `just check-changelog`, wired into `just ci-rust`) enforces keep-a-changelog header invariants and verifies ROADMAP.md declares at least one milestone.

## Release flow

Releases are automated by [release-plz](https://release-plz.dev/) and driven by Conventional Commits on `main`.

**Every PR merge:** the `release-plz` workflow opens (or updates) a single `chore(release): vX.Y.Z` PR with `workspace.package.version` bumped and new entries under `## [Unreleased]` in `CHANGELOG.md` grouped per keep-a-changelog 1.1.0.

**Cutting a release:**

1. Merge the release PR.
2. `release-plz-release` job runs on the post-merge push, tags `vX.Y.Z`, publishes each `publish = true` crate to crates.io, and opens a GitHub Release for the tag.
3. The existing `release.yml` workflow fires on the new tag and builds the cross-platform tarballs.

**Crates that publish in v0.1** (`release-plz.toml`): `stoa-core`, `stoa-recall`, `stoa-cli`, `stoa-hooks`, `stoa-inject-hooks`. `stoa-doclint` stays internal.

**Required repo secret:** `CARGO_REGISTRY_TOKEN` (crates.io API token with publish rights).

**Pre-pivot crates** previously published at 0.1.0 are obsolete. The yank + republish plan is in [ROADMAP.md](./ROADMAP.md) §M-v0.1.
