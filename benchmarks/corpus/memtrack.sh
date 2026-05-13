#!/usr/bin/env bash
# Dataset: MEMTRACK — 47 long-context multi-platform event-timeline instances
# Source:  Google Drive (no HuggingFace dataset card published).
#          File ID: 1ymMXmOIhCUcwC1WKOW8kioZgeYyrt-qe
# Paper:   https://arxiv.org/abs/2510.01353 (Patronus AI, NeurIPS 2025 SEA workshop)
# Blog:    https://www.patronus.ai/blog/memtrack
# Scorer:  No public scorer repo. Implement from paper Section IV (Correctness via
#          partial/approximate match + LLM-as-judge; Efficiency = tool calls;
#          Redundancy = unnecessary re-fetches).
# License: Verify per upstream
# Size:    < 100 MB
# Usage:   bash benchmarks/corpus/memtrack.sh
#
# Platforms (per paper): Slack, Linear, Gitea (self-hosted Git, not GitHub).
# Schema is platform-heterogeneous — events carry platform-specific fields
# (Slack: channel/sender/message; Linear: title/description/team/priority/lead;
# Git: filesystem-based, accessed via Gitea Docker container).
# Each instance carries an average of 3.2 questions (max 5) injected sequentially.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/memtrack"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"
GDRIVE_FILE_ID="1ymMXmOIhCUcwC1WKOW8kioZgeYyrt-qe"

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "memtrack: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v gdown &>/dev/null; then
    echo "memtrack: gdown not found — run: pip install gdown" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
echo "memtrack: downloading from Google Drive (file ${GDRIVE_FILE_ID})…" >&2
gdown --id "${GDRIVE_FILE_ID}" --output "${CACHE_DIR}/memtrack.zip"
unzip -o "${CACHE_DIR}/memtrack.zip" -d "${CACHE_DIR}/data"
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "memtrack: done" >&2
