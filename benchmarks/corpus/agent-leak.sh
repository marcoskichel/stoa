#!/usr/bin/env bash
# Dataset: AgentLeak — 1000 scenarios, 32-class attack taxonomy in 6 families
#          (F1 Prompt/Instruction, F2 Indirect/Tool-Surface, F3 Memory/Persistence,
#           F4 Multi-Agent Coordination, F5 Reasoning/CoT, F6 Evasion/Obfuscation)
# Source:  https://github.com/Privatris/AgentLeak (data lives in `agentleak_data/datasets/`)
#          The HuggingFace mirror `humain2/AgentLeak` only carries a README — the
#          real jsonl corpus ships in the GitHub repo. NOTE: published taxonomy
#          ships F1–F4 across 6 attack classes (504 benign + 496 attacker); the
#          remaining F5/F6 classes are reserved in the paper but not yet released.
# Paper:   https://arxiv.org/abs/2602.11510
# License: NOASSERTION (verify per upstream)
# Size:    ~3 MB
# Usage:   bash benchmarks/corpus/agent-leak.sh
#
# Channel codes per project DOCUMENTATION.md (canonical mapping):
#   C1 = final_output    C2 = inter_agent     C3 = tool_input
#   C4 = tool_output     C5 = memory_write    C6 = log           C7 = artifact
#
# Reproducibility: GH_REF is pinned to a full commit SHA so the corpus does
# not drift if upstream rewrites `main`. To bump, run
#   git ls-remote https://github.com/Privatris/AgentLeak HEAD
# and replace GH_REF + EXPECTED_VERSION (which embeds the short SHA).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/agent-leak"
VERSION_FILE="${CACHE_DIR}/.version"
GH_REPO="Privatris/AgentLeak"
GH_REF="6729179de10e6ee8d346f6467f09ca8b4b97acf7"
GH_REF_SHORT="${GH_REF:0:7}"
EXPECTED_VERSION="gh-${GH_REF_SHORT}"
DATA_PREFIX="agentleak_data/datasets"
FILES=(
    "scenarios_full_1000.jsonl"
    "scenarios_difficult_100.jsonl"
    "scenarios_base_100.jsonl"
    "smoke_test.jsonl"
    "traces_internal_channels.jsonl"
)

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "agent-leak: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v curl &>/dev/null; then
    echo "agent-leak: curl not found — required for GitHub raw fetch" >&2
    exit 1
fi

DEST="${CACHE_DIR}/data"
mkdir -p "${DEST}"
echo "agent-leak: downloading from GitHub (${GH_REPO}@${GH_REF_SHORT})…" >&2
for f in "${FILES[@]}"; do
    url="https://raw.githubusercontent.com/${GH_REPO}/${GH_REF}/${DATA_PREFIX}/${f}"
    echo "  • ${f}" >&2
    curl --fail --silent --show-error --location \
        --proto '=https' --proto-redir '=https' \
        -o "${DEST}/${f}" \
        "${url}"
done
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "${GH_REF}" > "${CACHE_DIR}/.commit"
echo "agent-leak: done (commit ${GH_REF_SHORT})" >&2
