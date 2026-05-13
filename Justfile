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

# Flag doc comments that restate the identifier.
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

dev:
    bacon clippy

watch:
    watchexec --exts rs,py,toml --no-vcs-ignore --restart -- just lint test

watch-py:
    cd python && uv run pytest -q --looponfail

# --------------------------------------------------------------------------
# Strict caps + supply chain
# --------------------------------------------------------------------------

file-length:
    ./scripts/check-file-length.sh crates
    ./scripts/check_lengths.py

# CHANGELOG + issue/PR-template invariants — every shipped milestone
# must appear; community on-ramp files must exist.
check-changelog:
    ./scripts/check-changelog.sh

# Docs site: build with --strict so broken links or undefined nav
# entries fail the build. Output lands in `target/docs-site` (gitignored).
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
# Daemon (development convenience)
# --------------------------------------------------------------------------

# Start the recall daemon under the Python workspace. Honors STOA_RECALLD_SOCKET.
daemon-start:
    cd python && uv run --package stoa-recalld stoa-recalld --foreground &

# Stop the recall daemon (sends SIGTERM via the pidfile if present).
daemon-stop:
    cargo run --quiet -p stoa-cli -- daemon stop || true

# Health-probe the daemon.
daemon-status:
    cargo run --quiet -p stoa-cli -- daemon status

# --------------------------------------------------------------------------
# Install for local dev
# --------------------------------------------------------------------------

# Full dev environment bootstrap (idempotent).
install:
    ./scripts/bootstrap.sh

# Install Stoa's binaries + sync Python sidecar. Assumes dev tools present.
install-dev:
    cargo install --path crates/stoa-cli --locked
    cargo install --path crates/stoa-hooks --locked
    cargo install --path crates/stoa-inject-hooks --locked
    cd python && uv sync --all-groups

# --------------------------------------------------------------------------
# Release: native or cross, tarballs into dist/.
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
        builder=(cargo build)
    else
        builder=(cross build)
    fi
    "${builder[@]}" --release --locked --target "$triple" -p stoa-cli
    "${builder[@]}" --release --locked --target "$triple" -p stoa-hooks
    "${builder[@]}" --release --locked --target "$triple" -p stoa-inject-hooks
    mkdir -p dist
    ext=""
    case "$triple" in *windows*) ext=".exe" ;; esac
    tar -czf "dist/stoa-${triple}.tar.gz" \
        -C "target/${triple}/release" \
        "stoa${ext}" "stoa-hook${ext}" "stoa-inject-hook${ext}"
    ls -lh "dist/stoa-${triple}.tar.gz"

release-verify target:
    #!/usr/bin/env bash
    set -euo pipefail
    just release {{target}}
    case "{{target}}" in
        linux-x86_64)        triple="x86_64-unknown-linux-gnu" ;;
        linux-aarch64)       triple="aarch64-unknown-linux-gnu" ;;
        windows-x86_64)      triple="x86_64-pc-windows-msvc" ;;
        macos-x86_64)        triple="x86_64-apple-darwin" ;;
        macos-aarch64)       triple="aarch64-apple-darwin" ;;
        *)                   triple="{{target}}" ;;
    esac
    ext=""
    case "$triple" in *windows*) ext=".exe" ;; esac
    tarball="dist/stoa-${triple}.tar.gz"
    expected=("stoa${ext}" "stoa-hook${ext}" "stoa-inject-hook${ext}")
    contents="$(tar -tzf "$tarball")"
    errors=0
    for bin in "${expected[@]}"; do
        if ! grep -qxF "$bin" <<< "$contents"; then
            echo "release-verify: ${tarball} missing binary: ${bin}" >&2
            errors=$((errors + 1))
        fi
    done
    if [ "$errors" -gt 0 ]; then
        echo "release-verify: ${errors} missing binary(ies)" >&2
        exit 1
    fi
    echo "release-verify: ${tarball} contains all expected binaries"

# --------------------------------------------------------------------------
# CI gates
# --------------------------------------------------------------------------

ci-rust: lint lint-docs file-length check-changelog
    cargo build --workspace --locked
    cargo test --workspace --locked

ci-python: docs
    cd python && uv sync --all-groups --locked
    cd python && uv run ruff check .
    cd python && uv run ruff format --check .
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
