#!/usr/bin/env bash
# Dataset: AgentLeak — 1000 scenarios, 32-class attack taxonomy in 6 families
#          (F1 Prompt/Instruction, F2 Indirect/Tool-Surface, F3 Memory/Persistence,
#           F4 Multi-Agent Coordination, F5 Reasoning/CoT, F6 Evasion/Obfuscation)
# Source:  https://huggingface.co/datasets/humain2/AgentLeak
# Paper:   https://arxiv.org/abs/2602.11510
# Repo:    https://github.com/Privatris/AgentLeak
# License: Verify per upstream
# Size:    < 5 MB
# Usage:   bash benchmarks/corpus/agent-leak.sh
#
# Channel codes per paper Section III-B (canonical taxonomy):
#   C4 = tool output    C5 = shared memory / agent state    C6 = system logs / telemetry
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/agent-leak"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
HF_HANDLE="humain2/AgentLeak"

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
