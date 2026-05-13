<!--
PR title MUST use Conventional Commits 1.0.0:
    <type>(<scope>)?: <subject>
Types: feat, fix, docs, refactor, perf, test, build, ci, chore, revert.
Breaking: append `!` and include a `BREAKING CHANGE:` footer.
Full guide: CONTRIBUTING.md §"Commits and PR titles".

The body sections below are visible in the GitHub PR preview — please fill
all three. Delete the HTML comments after you've read them.
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

- [ ] `just ci` passes locally
- [ ] Added or updated tests at `<path>`
- [ ] Manual verification:

## Risk + rollout

<!--
- Touches the hook hot path? If so, did the latency benchmark stay under
  10 ms p95?
- Changes the MINJA wrapper, the audit-log schema, or the on-disk wiki
  layout? Link the `ARCHITECTURE.md` section the change reflects.
- Safe to revert with a single `git revert`? If not, explain the cleanup
  story.
- Adds or removes a milestone heading in `ROADMAP.md`? `CHANGELOG.md` must
  match (gate: `just check-changelog`).
-->

## Linked issues

Closes #
