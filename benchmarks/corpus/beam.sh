#!/usr/bin/env bash
# Dataset: BEAM (Beyond a Million Tokens) — ICLR 2026
# Source:  https://arxiv.org/abs/2510.27246
#          HuggingFace handle TBD — confirm from the paper's dataset card.
# License: Verify before use.
# Size:    Variable by tier (128K / 500K / 1M / 10M). Download 128K+1M for v0.1.
# Usage:   bash benchmarks/corpus/beam.sh [--tier 128k|1m|10m]
#
# FIXME: Replace PLACEHOLDER_HF_HANDLE with the confirmed HuggingFace handle
#        from the BEAM paper's data release page.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/beam"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="PLACEHOLDER_HF_HANDLE"
TIER="${1:-1m}"

if [[ "${HF_HANDLE}" == "PLACEHOLDER_HF_HANDLE" ]]; then
    echo "beam: HuggingFace handle not yet confirmed — see FIXME in this script" >&2
    exit 1
fi

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
