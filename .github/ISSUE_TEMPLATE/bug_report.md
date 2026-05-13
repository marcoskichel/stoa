---
name: Bug report
about: Reproducible defect in stoa, stoa-hook, stoa-inject-hook, or the Python sidecar
title: "bug: <short description>"
labels: ["bug", "triage"]
---

## What happened

<!-- One sentence. What did you observe? -->

## What you expected

<!-- One sentence. What should have happened? -->

## Reproduction

Minimal steps a maintainer can run on a clean workspace:

```bash
# 1.
# 2.
# 3.
```

## Environment

- `stoa --version` output:
- OS + version:
- Rust toolchain (`rustc --version`):
- Python version (only if the bug is in the sidecar):
- Backend (`local-chroma-sqlite` is the v0.1 default):

## Logs / output

<!--
Paste the full output of the failing command. If the bug surfaces
through Claude Code, also attach the relevant slice of:
  - `.stoa/audit.log` (post-event log; redactor has already run)
  - `sessions/<session-id>.jsonl` (PRE-REDACTION TRANSCRIPT — see warning)

SECURITY: `sessions/*.jsonl` are the *input* to the redactor, not the
output. They can contain API keys, OAuth tokens, and personal paths
verbatim. Before pasting any session transcript, redact manually — the
in-repo regex set in `crates/stoa-capture` covers the common cases
post-capture, but it has NOT run on the file you read off disk. When in
doubt, attach a synthetic reproduction instead of the real transcript.
-->

```text
<paste here>
```

## Hypothesis (optional)

<!-- If you already have a guess, share it. Otherwise leave blank. -->
