# Contributing to Stoa

## Quick start

```bash
just install-dev              # cargo install stoa-cli + uv sync python sidecar
just ci                       # full local gate (lint + test + supply chain)
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

## Commits

Conventional-ish: `area: brief subject`. See `git log` for the prevailing style.

Never commit secrets. Never commit `target/`, `.venv/`, or `dist/` (covered by `.gitignore`).

## Release flow

1. Bump versions across `Cargo.toml` workspace and every `pyproject.toml`.
2. Tag `v0.X.Y` and push. `release.yml` builds and attaches the tarballs.
