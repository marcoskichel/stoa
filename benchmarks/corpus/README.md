# Benchmark corpora

Shared download scripts for all benchmark datasets. Data files are gitignored; only the scripts live here.

Adding a corpus:

1. Place download script as `corpus/<benchmark-name>.sh`.
2. Script writes to `corpus/<benchmark-name>/` (gitignored).
3. Script must be idempotent + offline-friendly (skip download if local cache valid).
4. Document license, size, and source URL at the top of the script.

The benchmark's own runner (under `crates/stoa-bench`) is responsible for reading from `corpus/<benchmark-name>/`. No corpus parsing in this directory.

Honesty discipline: no test-corpus changes without re-running every prior published backend and re-publishing all affected `results/`. See [../README.md](../README.md).
