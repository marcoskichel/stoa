#!/usr/bin/env bash
# Dataset: BEAM (Beyond a Million Tokens) — ICLR 2026
# Source:  https://huggingface.co/datasets/Mohammadta/BEAM (128K / 500K / 1M)
#          https://huggingface.co/datasets/Mohammadta/BEAM-10M (10M tier — separate)
# Paper:   https://arxiv.org/abs/2510.27246
# Repo:    https://github.com/mohammadtavakoli78/BEAM
# License: Verify per upstream
# Size:    Variable by tier. Distribution: 20×128K, 35×500K, 35×1M, 10×10M
#          (100 conversations, 2,000 questions total — 20 per conversation).
# Usage:   bash benchmarks/corpus/beam.sh [--tier 128k|500k|1m|10m]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/beam"
EXPECTED_VERSION="1.0.0"
TIER="${1:-1m}"

case "${TIER}" in
    10m) HF_HANDLE="Mohammadta/BEAM-10M" ;;
    128k|500k|1m) HF_HANDLE="Mohammadta/BEAM" ;;
    *) echo "beam: unknown tier '${TIER}' (expected 128k|500k|1m|10m)" >&2; exit 1 ;;
esac

TIER_CACHE_DIR="${CACHE_DIR}/${TIER}"
if [[ -f "${TIER_CACHE_DIR}/.version" ]] && \
   [[ "$(cat "${TIER_CACHE_DIR}/.version")" == "${EXPECTED_VERSION}" ]]; then
    echo "beam (${TIER}): cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "beam: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${TIER_CACHE_DIR}"
echo "beam (${TIER}): downloading from HuggingFace (${HF_HANDLE})…" >&2
huggingface-cli download "${HF_HANDLE}" \
    --repo-type dataset \
    --local-dir "${TIER_CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${TIER_CACHE_DIR}/.version"
echo "beam (${TIER}): done" >&2
