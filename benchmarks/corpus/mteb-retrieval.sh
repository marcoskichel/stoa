#!/usr/bin/env bash
# Dataset: MTEB/BEIR subset — representative BEIR corpora for embedding evaluation
# Source:  https://huggingface.co/BeIR
# License: Varies per dataset — check individual BEIR corpus cards.
# Size:    ~2 GB for the selected BEIR subsets (scifact, nfcorpus, fiqa).
# Usage:   bash benchmarks/corpus/mteb-retrieval.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/mteb-retrieval"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="1.0.0"

BEIR_SUBSETS=("BeIR/scifact" "BeIR/nfcorpus" "BeIR/fiqa")

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "mteb-retrieval: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if ! command -v huggingface-cli &>/dev/null; then
    echo "mteb-retrieval: huggingface-cli not found — run: pip install huggingface_hub[cli]" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
for subset in "${BEIR_SUBSETS[@]}"; do
    name="${subset##*/}"
    echo "mteb-retrieval: downloading ${subset}…" >&2
    huggingface-cli download "${subset}" \
        --repo-type dataset \
        --local-dir "${CACHE_DIR}/${name}"
done
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "mteb-retrieval: done" >&2
