set shell := ["bash", "-euo", "pipefail", "-c"]

default: build

# --------------------------------------------------------------------------
# Build
# --------------------------------------------------------------------------

build:
    cargo build --workspace --locked
    cd python && uv sync --all-groups

build-release:
    cargo build --workspace --release --locked

# --------------------------------------------------------------------------
# Test
# --------------------------------------------------------------------------

test:
    cargo test --workspace --locked
    cd python && uv run pytest -q

# End-to-end tests — quality gate for user-facing CLI behavior.
# Builds the `stoa` binary then runs trycmd + integration tests.
e2e:
    cargo test -p stoa-cli --test '*' --locked
    cargo test -p stoa-core --test '*' --locked

# Snapshot review / regen for trycmd golden files.
# Use after intentional output changes; review diff carefully.
e2e-review:
    TRYCMD=overwrite cargo test -p stoa-cli --test cli_cmd --locked

# --------------------------------------------------------------------------
# Lint / format
# --------------------------------------------------------------------------

lint:
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo fmt --check
    cd python && uv run ruff check .
    cd python && uv run ruff format --check .

# Flag doc comments that restate the identifier.
# Self-built tool — see crates/stoa-doclint.
lint-docs:
    cargo run --quiet --locked -p stoa-doclint -- crates

fmt:
    cargo fmt
    cd python && uv run ruff format .

typecheck:
    cd python && uv run basedpyright

# --------------------------------------------------------------------------
# Watch / dev loop
# Requires: `bacon` + `watchexec` (cargo install bacon watchexec-cli)
# --------------------------------------------------------------------------

# Interactive Rust dev loop (clippy on save).
dev:
    bacon clippy

# Cross-language watcher: re-run lint + test on any .rs/.py/.toml change.
watch:
    watchexec --exts rs,py,toml --no-vcs-ignore --restart -- just lint test

# Python-only test watcher.
watch-py:
    cd python && uv run pytest -q --looponfail

# --------------------------------------------------------------------------
# Strict caps + supply chain
# --------------------------------------------------------------------------

file-length:
    ./scripts/check-file-length.sh crates
    ./scripts/check_lengths.py

# CHANGELOG + issue/PR-template invariants — every shipped milestone
# (M0..Mn) must be referenced; community on-ramp files must exist.
check-changelog:
    ./scripts/check-changelog.sh

# Docs site: build with --strict so broken links or undefined nav
# entries fail the build. Uses the `docs` uv group; mkdocs.yml lives
# at the repo root, content under docs/. Output lands in
# `target/docs-site` (set in mkdocs.yml, gitignored).
docs:
    cd python && uv run --group docs mkdocs build --strict \
        --config-file ../mkdocs.yml

# Live-reload preview at http://127.0.0.1:8000 (authoring loop).
docs-serve:
    cd python && uv run --group docs mkdocs serve \
        --config-file ../mkdocs.yml --strict

deny:
    cargo deny --all-features check

machete:
    cargo machete --with-metadata

# --------------------------------------------------------------------------
# Benchmarks
# --------------------------------------------------------------------------

# Full v0.1 suite — requires M4 (LocalChromaSqliteBackend) to produce results.
bench:
    cargo run -p stoa-bench --release -- --backend local-chroma-sqlite

# Smoke run: validates fixtures exist and parse; does NOT require a live backend.
bench-smoke:
    cargo run -p stoa-bench --release -- --backend local-chroma-sqlite --smoke

# Download all v0.1 benchmark corpora into benchmarks/corpus/.
# Requires: huggingface-cli + gdown (pip install huggingface_hub[cli] gdown)
# MEMTRACK uses Google Drive (no HF dataset); BEAM 10M is a separate dataset.
bench-download-corpus:
    bash benchmarks/corpus/longmemeval.sh
    bash benchmarks/corpus/memory-agent-bench.sh
    bash benchmarks/corpus/memtrack.sh
    bash benchmarks/corpus/beam.sh 1m
    bash benchmarks/corpus/agent-leak.sh
    bash benchmarks/corpus/mteb-retrieval.sh

# Run a single benchmark by name (e.g. just bench-run longmemeval).
bench-run name:
    cargo run -p stoa-bench --release -- --backend local-chroma-sqlite --bench {{name}}

# --------------------------------------------------------------------------
# Install for local dev
# --------------------------------------------------------------------------

# Full dev environment bootstrap (idempotent).
# Installs dev tools + builds workspaces. Requires rustup + uv on PATH.
install:
    ./scripts/bootstrap.sh

# Install stoa CLI to ~/.cargo/bin + sync Python sidecar. Assumes dev tools
# already present (run `just install` first on a fresh clone).
install-dev:
    cargo install --path crates/stoa-cli --locked
    cd python && uv sync --all-groups

# --------------------------------------------------------------------------
# Release: native or cross, tarballs into dist/.
# Per M0 spike: linux/windows native cross or runner matrix; macOS via runner.
# --------------------------------------------------------------------------

release target:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
        linux-x86_64)        triple="x86_64-unknown-linux-gnu" ;;
        linux-aarch64)       triple="aarch64-unknown-linux-gnu" ;;
        windows-x86_64)      triple="x86_64-pc-windows-msvc" ;;
        macos-x86_64)        triple="x86_64-apple-darwin" ;;
        macos-aarch64)       triple="aarch64-apple-darwin" ;;
        *)                   triple="{{target}}" ;;
    esac
    host=$(rustc -vV | awk '/host:/{print $2}')
    if [ "$triple" = "$host" ]; then
        cargo build --release --locked --target "$triple" -p stoa-cli
        cargo build --release --locked --target "$triple" -p stoa-hooks
    else
        cross build --release --locked --target "$triple" -p stoa-cli
        cross build --release --locked --target "$triple" -p stoa-hooks
    fi
    mkdir -p dist
    ext=""
    case "$triple" in *windows*) ext=".exe" ;; esac
    tar -czf "dist/stoa-${triple}.tar.gz" \
        -C "target/${triple}/release" "stoa${ext}" "stoa-hook${ext}"
    ls -lh "dist/stoa-${triple}.tar.gz"

# --------------------------------------------------------------------------
# CI gates
# --------------------------------------------------------------------------

ci-rust: lint lint-docs file-length check-changelog
    cargo build --workspace --locked
    cargo test --workspace --locked
    just e2e

ci-python: docs
    cd python && uv sync --all-groups --locked
    cd python && uv run ruff check .
    cd python && uv run ruff format --check .
    cd python && uv run basedpyright
    ./scripts/check_lengths.py
    cd python && uv run pytest -q

ci: ci-rust ci-python deny

# --------------------------------------------------------------------------
# Convenience
# --------------------------------------------------------------------------

clean:
    cargo clean
    rm -rf dist python/.venv

version-cli:
    cargo run --quiet -p stoa-cli -- --version
