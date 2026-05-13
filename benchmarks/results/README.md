# Published results

CI-populated per-backend, per-benchmark results.

## File layout

```
results/<version>-<backend>-<benchmark>.md
```

Examples:

- `v0.1-local-chroma-sqlite-longmemeval.md`
- `v0.1-local-chroma-sqlite-memory-agent-bench.md`
- `v0.2-local-chroma-sqlite-beam.md`
- `v0.3-mempalace-longmemeval.md`

## Rules

- **Machine-generated only.** Manual edits forbidden. CI writes these files via `.github/workflows/bench.yml` (M4+).
- **No retroactive corrections.** If a number is wrong, the fix is a new run with a new commit, not an edit. Append a note pointing to the new file.
- **Every result includes**: backbone model + version, backend version, corpus commit hash, scorer commit hash, full hyperparameters, wall-clock + token cost.
- **Backend swap gate**: a new backend cannot merge until its results for every v0.1-tier benchmark are present here.

See [../README.md](../README.md) for the v0.1 benchmark suite and the cross-cutting honesty discipline.
