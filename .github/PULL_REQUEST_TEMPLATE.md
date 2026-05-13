<!--
Title format: <type>(<scope>)?: <subject>     (Conventional Commits)
Examples:
  feat(stoa-cli): add `stoa inject log --json`
  fix(stoa-hooks): drop runtime tokio dep from cold-start path
  docs(M6): wire CHANGELOG check into ci-rust

Types: feat, fix, docs, refactor, perf, test, build, ci, chore, revert.
Breaking change: append `!` and add a `BREAKING CHANGE:` footer in the
final commit message.
-->

## Summary

<!-- 1–3 bullets. The "why", not the "what" — the diff already shows the what. -->

-

## Test plan

<!--
Checklist of how a reviewer can verify the change. Include the exact
commands you ran. For UI / hook changes, describe the manual session
you exercised.
-->

- [ ] `just ci` green locally
- [ ] Added or updated tests at `<path>`
- [ ] Manual verification:

## Risk + rollout

<!--
- Does this touch the hook hot path? If so, did the latency benchmark
  stay under 10 ms p95?
- Does this change the MINJA wrapper, the audit log schema, or
  on-disk wiki layout? If so, link the ARCHITECTURE.md section the
  change reflects.
- Is this safe to revert with a single `git revert`? If not, explain
  the cleanup story.
-->

## Linked issues

Closes #
