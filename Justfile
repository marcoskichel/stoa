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

# --------------------------------------------------------------------------
# Lint / format
# --------------------------------------------------------------------------

lint:
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo fmt --check
    cd python && uv run ruff check .
    cd python && uv run ruff format --check .

fmt:
    cargo fmt
    cd python && uv run ruff format .

typecheck:
    cd python && uv run basedpyright

# --------------------------------------------------------------------------
# Strict caps + supply chain
# --------------------------------------------------------------------------

file-length:
    ./scripts/check-file-length.sh crates
    ./scripts/check_lengths.py

deny:
    cargo deny --all-features check

machete:
    cargo machete --with-metadata

# --------------------------------------------------------------------------
# Benchmarks
# --------------------------------------------------------------------------

bench:
    cargo run -p stoa-bench --release -- --backend local-chroma-sqlite

# --------------------------------------------------------------------------
# Install for local dev
# --------------------------------------------------------------------------

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

ci-rust: lint file-length
    cargo build --workspace --locked
    cargo test --workspace --locked

ci-python:
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
