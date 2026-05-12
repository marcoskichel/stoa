#!/usr/bin/env bash
# One-shot dev environment setup for a fresh clone.
# Idempotent: re-runs are safe.
#
# Prerequisites (curl-bash installers, kept manual for security):
#   - rustup: https://rustup.rs
#   - uv:     https://docs.astral.sh/uv/

set -euo pipefail

require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'ERROR: %s not found on PATH.\n  install: %s\n' "$1" "$2" >&2
        exit 1
    fi
}

require rustup "https://rustup.rs"
require uv     "https://docs.astral.sh/uv/"

echo "==> Installing Rust dev tools via cargo"
cargo install --locked \
    just \
    cargo-deny \
    cargo-machete \
    bacon \
    watchexec-cli \
    cross

echo "==> Building Rust workspace"
cargo build --workspace --locked

echo "==> Syncing Python workspace"
(cd python && uv sync --all-groups --locked)

echo
echo "Done. Verify with: just ci"
