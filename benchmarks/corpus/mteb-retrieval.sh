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
#
# Each subset is pinned to a commit SHA via `--revision`. Upstream mirrors
# occasionally rewrite history (file renames, schema tweaks); without a
# revision pin a re-download would silently move the bench off its
# baseline. The resolved SHAs are written to `<cache>/.version` so the
# Rust adapter can fold them into the result's `corpus_rev` later.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/mteb-retrieval"
VERSION_FILE="${CACHE_DIR}/.version"

SCIFACT_SHA="cf10ab6856b15b0e670ef8ae5dae4e266c12d035"
NFCORPUS_SHA="52ac3f19d3449632d9f00aab0ad34a110fc03816"
FIQA_SHA="5e59eeb3a7df6b85882112b747008547c21587ea"

EXPECTED_VERSION="2.1.0:scifact@${SCIFACT_SHA};nfcorpus@${NFCORPUS_SHA};fiqa@${FIQA_SHA}"

declare -A MTEB_SHAS=(
    [scifact]="${SCIFACT_SHA}"
    [nfcorpus]="${NFCORPUS_SHA}"
    [fiqa]="${FIQA_SHA}"
)
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
    sha="${MTEB_SHAS[${name}]}"
    if [[ -f "${target_dir}/corpus.jsonl" && -f "${target_dir}/queries.jsonl" && -f "${target_dir}/qrels/test.tsv" ]]; then
        echo "mteb-retrieval: ${name} already present" >&2
        continue
    fi
    echo "mteb-retrieval: downloading mteb/${name}@${sha}…" >&2
    "${DL_CMD[@]}" "mteb/${name}" \
        corpus.jsonl queries.jsonl qrels/test.tsv \
        --repo-type dataset \
        --revision "${sha}" \
        --local-dir "${target_dir}"
done
echo "${EXPECTED_VERSION}" > "${VERSION_FILE}"
echo "mteb-retrieval: done" >&2
