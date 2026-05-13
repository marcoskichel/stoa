#!/usr/bin/env bash
# Dataset: LongMemEval (Wu et al., 2024 — ICLR 2025)
# Source:  https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned
#          (cleaned split — fixes oracle JSON load + removes noisy sessions;
#           the original handle `xiaowu0162/longmemeval` has a broken viewer.)
# Paper:   https://arxiv.org/abs/2410.10813
# Scorer:  https://github.com/xiaowu0162/LongMemEval — src/evaluation/evaluate_qa.py
#          No release tags; pin to a main HEAD commit SHA.
# License: MIT
# Size:    ~3 GB
# Usage:   bash benchmarks/corpus/longmemeval.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/longmemeval"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="cleaned-2026-01"
HF_HANDLE="xiaowu0162/longmemeval-cleaned"

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
