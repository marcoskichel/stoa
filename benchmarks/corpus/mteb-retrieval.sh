#!/usr/bin/env bash
# Dataset: MTEB/BEIR subset — representative BEIR corpora for embedding evaluation
# Source:  Hugging Face mirror `mteb/<name>` (preserves BEIR's canonical
#          corpus.jsonl + queries.jsonl + qrels/test.tsv layout). The original
#          `BeIR/<name>` mirrors have migrated to parquet, which would force a
#          heavy arrow dependency on `stoa-bench`; `mteb/<name>` is the same
#          data in the original shape.
# License: Varies per dataset — check individual BEIR / mteb corpus cards.
# Size:    ~25 MB total for scifact + nfcorpus + fiqa.
# Usage:   bash benchmarks/corpus/mteb-retrieval.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/mteb-retrieval"
VERSION_FILE="${CACHE_DIR}/.version"
EXPECTED_VERSION="2.0.0"

MTEB_SUBSETS=("scifact" "nfcorpus" "fiqa")

if [[ -f "${VERSION_FILE}" ]] && [[ "$(cat "${VERSION_FILE}")" == "${EXPECTED_VERSION}" ]]; then
    echo "mteb-retrieval: cache valid (${EXPECTED_VERSION})" >&2
    exit 0
fi

if command -v hf &>/dev/null; then
    DL_CMD=(hf download)
elif command -v huggingface-cli &>/dev/null; then
    DL_CMD=(huggingface-cli download)
else
    echo "mteb-retrieval: hf CLI not found — run: pip install -U huggingface_hub" >&2
    exit 1
fi

mkdir -p "${CACHE_DIR}"
for name in "${MTEB_SUBSETS[@]}"; do
    target_dir="${CACHE_DIR}/${name}"
    if [[ -f "${target_dir}/corpus.jsonl" && -f "${target_dir}/queries.jsonl" && -f "${target_dir}/qrels/test.tsv" ]]; then
        echo "mteb-retrieval: ${name} already present" >&2
        continue
    fi
    echo "mteb-retrieval: downloading mteb/${name}…" >&2
    "${DL_CMD[@]}" "mteb/${name}" \
        corpus.jsonl queries.jsonl qrels/test.tsv \
        --repo-type dataset \
        --local-dir "${target_dir}"
done
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "mteb-retrieval: done" >&2
