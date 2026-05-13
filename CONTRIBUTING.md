# Contributing to Stoa

## Quick start

Fresh clone (assumes `rustup` + `uv` already installed):

```bash
./scripts/bootstrap.sh        # installs dev tools + builds both workspaces
just ci                       # full local gate (lint + test + supply chain)
```

Once dev tools are present, day-to-day:

```bash
just install-dev              # cargo install stoa-cli + uv sync sidecar
just dev                      # interactive Rust dev loop (bacon)
just watch                    # cross-language lint+test on save
```

## Layout

See [ARCHITECTURE.md §16.2](./ARCHITECTURE.md). Crates live under `crates/`, Python sidecar packages under `python/packages/`.

## Toolchain

Pinned via `rust-toolchain.toml` (stable 1.95) and `python/pyproject.toml` (Python 3.13). Install:

- [`rustup`](https://rustup.rs) — toolchain manager
- [`uv`](https://docs.astral.sh/uv/) — Python env manager
- [`just`](https://just.systems) — task runner
- [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) — license + advisory + sources gate
- [`cargo-machete`](https://github.com/bnjbvr/cargo-machete) — unused-dep finder
- [`cross`](https://github.com/cross-rs/cross) — only needed for non-native Rust release builds
- [`bacon`](https://dystroy.org/bacon/) — interactive Rust dev loop (`just dev`)
- [`watchexec`](https://watchexec.github.io/) — cross-language file watcher (`just watch`)

## Coding rules

The lint configs are strict by design. See `Cargo.toml` workspace lints, `clippy.toml`, `rustfmt.toml` (Rust); `python/pyproject.toml` `[tool.ruff]` and `[tool.basedpyright]` (Python).

Highlights:

- **Function body ≤ 25 lines**, **file ≤ 400 lines**. Enforced by `clippy::too_many_lines` + `scripts/check-file-length.sh` (Rust) and Pylint `max-statements` + `scripts/check_lengths.py` (Python).
- **Rust**: `unsafe_code = forbid` at workspace level. Override per-crate with a `reason = "..."`. No `unwrap`/`expect`/`panic`/`todo`/`dbg!` in non-test code.
- **Python**: full type coverage via `basedpyright --strict`; `Any` is an error, including the `reportAny` rule.
- **Escape hatches**: use `#[expect(<lint>, reason = "...")]` in Rust and `# type: ignore[<rule>]` with explicit rule code in Python. Never bare `#[allow]` or `# noqa`.
- Imports: `from __future__ import annotations` is required in every Python module.
- **Comment policy** (`just lint-docs`, runs `stoa-doclint`). Doc comments — `///`, `//!`, `/** */`, `/*! */` — are always allowed; they survive in `rustdoc`. Bare `//` comments are forbidden unless the line opens with one of six durable intent prefixes — `SAFETY:`, `FIXME:`, `HACK:`, `PERF:`, `NOTE:`, `WHY:`. `TODO:` is intentionally not an allowed prefix: track TODOs in the issue tracker so they have an owner. Non-doc `/* */` block comments are forbidden outright.
- **Doc-comment content.** Doc comments describe how the thing works — invariants, edge cases, error conditions, non-obvious constraints, performance characteristics callers depend on. They are **not** a place for transient information: no milestone or roadmap pointers (`M1 skeleton — lands in M3`), no TODO lists, no implementation history, no authorship or dates. If the only thing a doc comment adds is the identifier rephrased in prose, delete it.

## Environment traps documented during M0 spike

These bit the spike author once; documented so contributors hit them at most once.

1. **System Rust vs rustup on Arch Linux (and other distros that ship Rust as a system package).** Arch's `/usr/bin/cargo` (pacman) shadows `~/.cargo/bin/cargo` (rustup) in `PATH`. `cross-rs` reads rustup metadata to resolve the active toolchain; with system Rust active it fails with `invalid toolchain name: 'usr'`. Fix:

   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   export RUSTUP_TOOLCHAIN=stable
   ```

   CI uses rustup-managed toolchains, so this is local-only.

2. **Wine in cross-rs windows-gnu image on Linux ≥ 7.x.** `wineboot` fails inside the container with `wine: socket : Function not implemented` under the default Docker seccomp profile. Fix:

   ```bash
   export CROSS_CONTAINER_OPTS="--security-opt seccomp=unconfined"
   ```

   Production CI builds Windows natively on a `windows-latest` runner, so this only matters for local cross-builds.

## CI

Three workflows:

- `.github/workflows/rust.yml` — fmt + clippy + build + test on Linux/macOS/Windows; cargo-deny + cargo-machete on Linux.
- `.github/workflows/python.yml` — ruff lint+format check + basedpyright + length caps + pytest on Linux/macOS.
- `.github/workflows/release.yml` — runs on tag push; builds release tarballs across 5 targets via a runner matrix (per M0 decision: macOS uses `macos-latest` rather than cross).

## Commits and PR titles

Use [Conventional Commits 1.0.0](https://www.conventionalcommits.org/) for
**both commit messages and PR titles**: `<type>(<scope>)?: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`,
  `chore`, `revert`. Breaking changes append `!` (e.g. `feat!: ...`) and
  include a `BREAKING CHANGE:` footer.
- Scope is optional but encouraged when work is crate- or milestone-bound:
  `feat(stoa-cli): ...`, `chore(M6): ...`.
- Subject: imperative mood, no trailing period, ≤72 chars.

The `.github/PULL_REQUEST_TEMPLATE.md` repeats the format as an HTML
comment at the top so authors see it while drafting; the rules live here
in `CONTRIBUTING.md` so they are reachable from the issue-template contact
links.

Never commit secrets. Never commit `target/`, `.venv/`, or `dist/` (covered
by `.gitignore`).

## Changelog

`CHANGELOG.md` follows
[keep-a-changelog 1.1.0](https://keepachangelog.com/en/1.1.0/):

- Keep an `## [Unreleased]` section at the top of the file *at all times*.
  After tagging a release, do **not** rename `[Unreleased]` — add a new
  `## [<version>] - <YYYY-MM-DD>` section *below* it and leave
  `[Unreleased]` empty.
- `scripts/check-changelog.sh` (run by `just check-changelog`, wired into
  `just ci-rust`) derives the required-milestone list from `ROADMAP.md`.
  When you add a new milestone heading there, the gate requires a
  matching `Mn` reference in `CHANGELOG.md` automatically — no script
  edit needed.

## Release flow

Releases are automated by [release-plz](https://release-plz.dev/) and
driven entirely by the Conventional Commits on `main`.

**Every PR merge:**

The `release-plz` workflow opens (or updates) a single
`chore(release): vX.Y.Z` PR with:

- `workspace.package.version` bumped in `Cargo.toml` (every crate
  inherits via `version.workspace = true`).
- New entries under `## [Unreleased]` in `CHANGELOG.md` grouped per
  keep-a-changelog 1.1.0 (Added / Changed / Fixed / Removed /
  Security). The grouping is derived from each commit's Conventional
  Commits type.

Edit the PR in-place to reword changelog prose if needed — release-plz
only rewrites the entries it generated.

**Cutting a release:**

1. Merge the release PR.
2. The `release-plz-release` job runs on the post-merge push, tags
   `vX.Y.Z`, publishes each `publish = true` crate to crates.io, and
   opens a GitHub Release for the tag.
3. The existing `release.yml` workflow fires on the new tag and
   builds the cross-platform tarballs.

**Internal crates that do not publish** (`stoa-bench`, `stoa-doclint`,
`stoa-render-mermaid`, `stoa-render-svg`, `stoa-render-tui`,
`stoa-viz`) are opted out in `release-plz.toml`. They still bump
version + changelog in lockstep with the rest of the workspace; they
just skip `cargo publish` and the per-crate GitHub Release.

**Required repo secret:** `CARGO_REGISTRY_TOKEN` (a crates.io API
token with publish rights). `GITHUB_TOKEN` is provided automatically.

**Manual fallback** (release-plz down / first-tag bootstrap):

1. Bump `workspace.package.version` in `Cargo.toml`. Every crate
   inherits via `version.workspace = true`.
2. Bump `version =` in every `python/packages/*/pyproject.toml` — the
   Python sidecar lives outside the Cargo workspace so release-plz
   does not touch it. Sidecar versions are pinned manually until the
   v0.3 rewrite deletes the sidecar entirely.
3. Update `CHANGELOG.md` per keep-a-changelog 1.1.0: insert a new
   `## [<version>] - <YYYY-MM-DD>` section *below* `[Unreleased]`,
   move all `[Unreleased]` entries into it, leave `[Unreleased]`
   empty. Do **not** rename `[Unreleased]`.
4. Commit, tag `vX.Y.Z`, push. `release.yml` still builds the
   cross-platform tarballs. `cargo publish` each `publish = true`
   crate by hand (`stoa-core`, `stoa-queue`, `stoa-capture`,
   `stoa-recall`, `stoa-recall-local-chroma-sqlite`, `stoa-cli`,
   `stoa-hooks`, `stoa-inject-hooks`).

**Partial-state recovery** (release-plz cuts a tag then fails before
`cargo publish`): delete the new tag locally and on GitHub
(`git tag -d vX.Y.Z && git push origin :refs/tags/vX.Y.Z`), delete
the GitHub Release at https://github.com/marcoskichel/stoa/releases,
revert the release PR's merge commit, re-trigger the release flow.
The `Require CARGO_REGISTRY_TOKEN` pre-flight step in
`.github/workflows/release-plz.yml` is designed to make this path
unreachable in normal operation.
