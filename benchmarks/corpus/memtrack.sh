#!/usr/bin/env bash
# Dataset: MEMTRACK (Multi-platform Event-Timeline Tracking)
# Source:  HuggingFace handle TBD — confirm from paper authors before running.
#          47 expert-curated scenarios across Slack / Linear / Git.
# License: Verify before use.
# Size:    < 1 MB (47 scenarios)
# Usage:   bash benchmarks/corpus/memtrack.sh
#
# FIXME: Replace PLACEHOLDER_HF_HANDLE with the correct HuggingFace dataset
#        handle once confirmed from the MEMTRACK paper repository.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/memtrack"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="PLACEHOLDER_HF_HANDLE"

if [[ "${HF_HANDLE}" == "PLACEHOLDER_HF_HANDLE" ]]; then
    echo "memtrack: HuggingFace handle not yet confirmed — see FIXME in this script" >&2
    exit 1
fi

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "memtrack: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "memtrack: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
echo "memtrack: downloading from HuggingFace (${HF_HANDLE})…" >&2
huggingface-cli download "${HF_HANDLE}" \
    --repo-type dataset \
    --local-dir "${CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "memtrack: done" >&2
