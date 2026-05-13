#!/usr/bin/env bash
# Quality gate: CHANGELOG.md must follow keep-a-changelog 1.1.0 and
# document every milestone shipped to date (M0..M5). Failing this gate
# blocks the v0.1 release because the public history must match the
# git history.
#
# Run: ./scripts/check-changelog.sh
# Exit 0 on success, non-zero on the first failure (with diagnostic).

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
changelog="${repo_root}/CHANGELOG.md"

fail() {
    echo "check-changelog: $*" >&2
    exit 1
}

[ -f "$changelog" ] || fail "missing CHANGELOG.md at repo root"

# keep-a-changelog header invariants.
grep -q '^# Changelog' "$changelog" || fail "CHANGELOG.md must open with '# Changelog' header"
grep -q 'keepachangelog' "$changelog" || fail "CHANGELOG.md must reference keepachangelog.com (format link)"
grep -q 'semver' "$changelog" || grep -q 'Semantic Versioning' "$changelog" \
    || fail "CHANGELOG.md must reference Semantic Versioning"

# Must carry an [Unreleased] section since v0.1 is not yet tagged.
grep -q '^## \[Unreleased\]' "$changelog" \
    || fail "CHANGELOG.md must have an '## [Unreleased]' section before v0.1.0 is tagged"

# Every milestone shipped (M0..M5) must be referenced. The grep is loose
# (case-insensitive, allows 'M3' or 'M3 —') because the format is the
# author's call, but the substring must appear so a reader can find it.
for m in M0 M1 M2 M3 M4 M5; do
    grep -qiE "\\b${m}\\b" "$changelog" \
        || fail "CHANGELOG.md does not mention milestone ${m} — every shipped milestone must be documented"
done

# Issue + PR templates must exist alongside the changelog so the
# 'community-on-ramp' deliverable in M6 ships as one coherent unit.
template_dir="${repo_root}/.github/ISSUE_TEMPLATE"
[ -d "$template_dir" ] || fail "missing .github/ISSUE_TEMPLATE/ directory"
required_templates=(bug_report.md feature_request.md config.yml)
for t in "${required_templates[@]}"; do
    [ -f "${template_dir}/${t}" ] || fail "missing .github/ISSUE_TEMPLATE/${t}"
done
[ -f "${repo_root}/.github/PULL_REQUEST_TEMPLATE.md" ] \
    || fail "missing .github/PULL_REQUEST_TEMPLATE.md"

echo "check-changelog: ok"
