#!/usr/bin/env bash
# Quality gate: CHANGELOG.md must follow keep-a-changelog 1.1.0 and
# document every milestone defined in ROADMAP.md. Failing this gate
# blocks the v0.1 release because the public history must match the
# documented milestone plan.
#
# Run: ./scripts/check-changelog.sh
# Behavior: collects every violation, prints one line per violation,
# exits non-zero if any were found. Resolves the repo root from this
# script's location so it works under any caller cwd.

set -uo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
changelog="${repo_root}/CHANGELOG.md"
roadmap="${repo_root}/ROADMAP.md"

errors=0
fail() {
    echo "check-changelog: $*" >&2
    errors=$((errors + 1))
}

if [ ! -f "$changelog" ]; then
    fail "missing CHANGELOG.md at repo root"
    exit 1
fi

# keep-a-changelog header invariants.
grep -q '^# Changelog' "$changelog" || fail "CHANGELOG.md must open with '# Changelog' header"
grep -q 'keepachangelog' "$changelog" || fail "CHANGELOG.md must reference keepachangelog.com (format link)"
grep -qiE 'semver|Semantic Versioning' "$changelog" \
    || fail "CHANGELOG.md must reference Semantic Versioning"

# Keep-a-changelog 1.1.0 convention: keep an [Unreleased] section at
# the top of the file at all times — additions go there until cut.
# After tagging, do NOT rename [Unreleased]; instead add a new
# `## [<version>] - <date>` section *below* it. See CONTRIBUTING.md
# §"Release flow" for the procedure.
grep -q '^## \[Unreleased\]' "$changelog" \
    || fail "CHANGELOG.md must keep an '## [Unreleased]' section (keep-a-changelog 1.1.0)"

# Derive the required milestone list from ROADMAP.md so the gate
# does not silently rot when a new milestone is added. Falls back to
# the hardcoded MVP list if ROADMAP.md is unreadable (treated as a
# defect because the gate becomes opaque).
if [ -f "$roadmap" ]; then
    milestones="$(grep -oE '^### M[0-9]+' "$roadmap" | grep -oE 'M[0-9]+' | sort -u)"
fi
if [ -z "${milestones:-}" ]; then
    fail "could not derive milestone list from ROADMAP.md — gate cannot run"
    milestones="M0 M1 M2 M3 M4 M5 M6"
fi
for m in $milestones; do
    grep -qiE "\\b${m}\\b" "$changelog" \
        || fail "CHANGELOG.md does not mention milestone ${m} (defined in ROADMAP.md)"
done

# Issue + PR templates must exist alongside the changelog so the
# community on-ramp ships as one coherent unit.
template_dir="${repo_root}/.github/ISSUE_TEMPLATE"
if [ ! -d "$template_dir" ]; then
    fail "missing .github/ISSUE_TEMPLATE/ directory"
else
    required_templates=(bug_report.md feature_request.md config.yml)
    for t in "${required_templates[@]}"; do
        [ -f "${template_dir}/${t}" ] || fail "missing .github/ISSUE_TEMPLATE/${t}"
    done
fi
[ -f "${repo_root}/.github/PULL_REQUEST_TEMPLATE.md" ] \
    || fail "missing .github/PULL_REQUEST_TEMPLATE.md"

if [ "$errors" -gt 0 ]; then
    echo "check-changelog: $errors violation(s)" >&2
    exit 1
fi
echo "check-changelog: ok"
