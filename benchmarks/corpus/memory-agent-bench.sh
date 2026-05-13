#!/usr/bin/env bash
# Dataset: MemoryAgentBench
# Source:  https://huggingface.co/datasets/emrecanacikgoz/MemoryAgentBench
# License: Apache-2.0 (verify before use)
# Size:    ~10 MB
# Usage:   bash benchmarks/corpus/memory-agent-bench.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/memory-agent-bench"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="emrecanacikgoz/MemoryAgentBench"

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "memory-agent-bench: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "memory-agent-bench: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
echo "memory-agent-bench: downloading from HuggingFace (${HF_HANDLE})…" >&2
huggingface-cli download "${HF_HANDLE}" \
    --repo-type dataset \
    --local-dir "${CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "memory-agent-bench: done" >&2
