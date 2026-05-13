#!/usr/bin/env bash
# Dataset: AgentLeak — 32-class PII leak taxonomy across 7 channel classes
# Source:  HuggingFace handle TBD — confirm from paper authors.
# License: Verify before use.
# Size:    < 5 MB
# Usage:   bash benchmarks/corpus/agent-leak.sh
#
# FIXME: Replace PLACEHOLDER_HF_HANDLE with the confirmed HuggingFace handle
#        from the AgentLeak paper data release.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/agent-leak"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="PLACEHOLDER_HF_HANDLE"

if [[ "${HF_HANDLE}" == "PLACEHOLDER_HF_HANDLE" ]]; then
    echo "agent-leak: HuggingFace handle not yet confirmed — see FIXME in this script" >&2
    exit 1
fi

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "agent-leak: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "agent-leak: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
echo "agent-leak: downloading from HuggingFace (${HF_HANDLE})…" >&2
huggingface-cli download "${HF_HANDLE}" \
    --repo-type dataset \
    --local-dir "${CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "agent-leak: done" >&2
