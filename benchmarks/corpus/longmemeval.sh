#!/usr/bin/env bash
# Dataset: LongMemEval (Wu et al., 2024)
# Source:  https://huggingface.co/datasets/xiaowu0162/longmemeval
# License: CC-BY-4.0
# Size:    ~50 MB compressed
# Usage:   bash benchmarks/corpus/longmemeval.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/longmemeval"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="xiaowu0162/longmemeval"

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "longmemeval: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "longmemeval: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
echo "longmemeval: downloading from HuggingFace (${HF_HANDLE})…" >&2
huggingface-cli download "${HF_HANDLE}" \
    --repo-type dataset \
    --local-dir "${CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "longmemeval: done" >&2
