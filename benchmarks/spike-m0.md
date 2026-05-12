# M0 — Validation spike report

**Status**: COMPLETE
**Date**: 2026-05-12
**Goal**: Validate three load-bearing assumptions in [ARCHITECTURE.md §15](../ARCHITECTURE.md) before any v0.1 implementation begins. Per [ROADMAP.md sequencing rule #1](../ROADMAP.md#sequencing-rules-load-bearing), nothing else starts until this report ships.

## TL;DR

| assumption | verdict | risk to v0.1 |
|---|---|---|
| 1. hook latency <10ms p95 | ✅ GREEN (Linux measured; macOS pending hardware) | low |
| 2. fastembed Rust↔Python parity + throughput | ✅ GREEN | low |
| 3. cross-compile to 5 release targets | 🟡 PARTIAL — 3/5 native+cross; macOS needs CI runner or custom osxcross image | low (well-known pattern) |

**Green-light decision**: ✅ **Proceed to M1.** Core architectural assumptions hold. The macOS cross-compile gap is solved by a well-understood CI pattern, not by an architectural change. No revisions to [ARCHITECTURE.md §15](../ARCHITECTURE.md) required.

---

## Test environment

| | |
|--|--|
| OS | Arch Linux, kernel 7.0.3-arch1-2 |
| CPU | AMD Ryzen 7 5800X3D 8-core / 16-thread |
| RAM | 31 GiB |
| Rust | stable 1.95.0 (rustup-managed) |
| cross | 0.2.5 (crates.io) |
| docker | 29.4.2 |
| uv | 0.11.11 |
| Python | 3.13 |

**Caveat**: macOS hardware not available. Hook latency on Apple Silicon is *not* measured here; that has to land before v0.1 ships. macOS cross-compile is attempted but reflects host-toolchain availability, not Apple-Silicon runtime behavior.

---

## Assumption 1 — Hook cold-start <10ms p95

**Claim**: A stripped Rust binary that opens `.stoa/queue.db` (WAL + `synchronous=NORMAL`), inserts one row, and exits will complete in <10ms p95 on commodity hardware. This is the architectural budget that makes passive capture invisible to interactive editor sessions.

**Method**:
- Spike binary: 116 LOC (`benchmarks/spike-m0/hook-latency/src/main.rs`).
- Deps: `rusqlite v0.38` with `bundled` feature (no system libsqlite3 dependency).
- Release profile: `lto=thin, codegen-units=1, strip=true, panic=abort`.
- Stripped binary size: **2.4 MB** (well within an OS page-cache working set).
- Measurement: `date +%s%N` wall-clock around the binary invocation (includes fork/exec, dynamic linker, all I/O, exit). 1000 runs hot-cache, 200 runs with fresh inode (best approximation of cold without `drop_caches` root).

**Result**:

| metric | hot cache (n=1000) | fresh inode (n=200) |
|---|---:|---:|
| min | 1.89 ms | 1.89 ms |
| p50 | 2.09 ms | 2.14 ms |
| p95 | 2.30 ms | 2.32 ms |
| p99 | 2.36 ms | 2.37 ms |
| max | 2.40 ms | 2.41 ms |
| mean | 2.11 ms | 2.14 ms |
| stdev | 0.11 ms | 0.12 ms |

**Verdict**: ✅ **GREEN**. Linux p95 = 2.3 ms — under one-quarter of the 10 ms budget. Variance is tight (σ = 0.12 ms). No outliers approach the budget.

**Remaining risk**:
- macOS not measured. Apple Silicon process spawn is generally faster than Linux x86_64; risk of regression is low but not zero. Required follow-up before M3 merges: rerun on macOS hardware as part of M1.
- Truly-cold (post-`drop_caches`) latency not measured because the spike runs without root. The fresh-inode result is an upper-bound proxy and tracks hot cache within noise — first-ever invocation after a fresh boot may be ~1 ms slower from binary page-fault cost. Even with 2× headroom this stays under 5 ms.

**Implication for ROADMAP**: M3's "<10 ms p95 CI gate" is achievable with comfortable headroom. The budget protects future regressions (heavier dependencies, redaction overhead, additional schema work) rather than constraining the current design.

---

## Assumption 2 — `fastembed` (Rust) parity with Python reference

**Claim**: `fastembed` v5 in Rust running `bge-small-en-v1.5` via ONNX Runtime produces embeddings indistinguishable from the Python reference, with throughput sufficient for the v0.2 migration plan in §15.

**Method**:
- Fixed corpus: 12 short technical sentences (`benchmarks/spike-m0/fastembed-parity/src/main.rs`, `embed_python.py`).
- Both implementations run `bge-small-en-v1.5`, batch then per-text.
- Cosine similarity computed pairwise between Rust and Python embeddings.
- Throughput measured after one warm-up batch.

**Result**:

| | Rust (`fastembed v5.13.4`, `ort v2.0.0-rc.12`) | Python (`fastembed`, ONNX) |
|---|---:|---:|
| Embedding dim | 384 | 384 |
| Batch (12 texts) | 25 ms (480 texts/sec) | 18 ms (660 texts/sec) |
| Per-text avg | 3.5 ms | 2.0 ms |
| Model load (warm) | 255 ms | 170 ms |
| Stripped binary | 32.5 MB | n/a |

**Per-text Rust↔Python cosine similarity** (n=12):

| min | mean | max |
|---:|---:|---:|
| 0.999998 | 0.999999 | 0.999999 |

All 12 pairs agree to ≥6 decimal places. Difference is well within ONNX Runtime numerical noise (different op-fusion schedules between platforms).

**Verdict**: ✅ **GREEN on parity**. ✅ **GREEN on throughput** for v0.2 migration target. Python is ~37% faster on this CPU corpus (likely Python-side batching pipeline tuning), but Rust at 480 texts/sec is far above the SessionStart hot-path requirement (typical: 1–10 query embeddings → <40 ms).

**Remaining risk**:
- 32.5 MB Rust binary (with bundled `ort` + `tokenizers`) is acceptable for `cargo install` distribution but will require attention if larger models land. Document the size budget in §15 when v0.2 begins.
- Rust throughput gap vs Python (~1.4×) is worth investigating during the v0.2 spike; root-cause is likely per-call setup overhead in the fastembed Rust API surface, not raw ONNX execution speed.
- `ort v2.0.0-rc.12` is still a release candidate. Track GA before committing to the v0.2 migration. If `ort` slips, `tract` (pure-Rust ONNX) is the documented fallback in §15.

**Implication for ROADMAP**: v0.1 ships the Python sidecar as planned (per §15 Shape A). The v0.2 milestone "embedding worker → Rust" has no blocker from the parity dimension; only `ort` GA-status to monitor.

---

## Assumption 3 — `cross` cross-compile to all 5 release targets

**Claim**: The v0.1 dependency set (rusqlite bundled; no embedding inference; the binary used for the hook spike) cross-compiles from a Linux x86_64 host to all 5 release targets using `cross-rs`.

**Method**:
- `cross v0.2.5` (crates.io) + Docker 29.4.2.
- Build script: `benchmarks/spike-m0/cross-compile/build-all.sh`.
- Native build for `x86_64-unknown-linux-gnu`; `cross build` for the other four targets.
- Release profile identical to spike 1.

**Result**:

| target | status | binary size | duration | notes |
|---|---|---:|---:|---|
| `x86_64-unknown-linux-gnu` | ✅ OK | 2.47 MB | 44 s | native |
| `aarch64-unknown-linux-gnu` | ✅ OK | 1.76 MB | 27 s | cross + linux-aarch64 image |
| `x86_64-pc-windows-gnu` | ✅ OK | 1.79 MB | 31 s | cross + mingw image |
| `x86_64-apple-darwin` | ❌ FAIL | — | <1 s | no osxcross image |
| `aarch64-apple-darwin` | ❌ FAIL | — | <1 s | no osxcross image |

**Verdict**: 🟡 **PARTIAL — 3/5 succeed; macOS targets fail with a known, well-documented cause.**

**Failure mode for macOS**: `cross-rs` does not ship a Docker image for `*-apple-darwin` because the macOS SDK license forbids redistribution. With no image, `cross` falls back to the host gcc, which doesn't accept Apple-specific flags (`-arch`, `-mmacosx-version-min`). This is documented in cross-rs README; the resolution is one of:
  1. **Recommended for v0.1**: GitHub Actions `macos-latest` runner does native macOS builds — no cross-compile needed at all.
  2. Build a custom osxcross Docker image locally (~1–2 hour setup, license-grey).
  3. Use Apple's own toolchain on a macOS host.

**Two non-architectural environment issues found and fixed during the spike** (documented for M1):

1. **System Rust vs rustup**: Arch's `/usr/bin/cargo` (pacman package) shadowed `~/.cargo/bin/cargo` (rustup) in PATH. cross-rs depends on rustup metadata to resolve the active toolchain; with system Rust active it failed with `invalid toolchain name: 'usr'`. Fix: `export PATH="$HOME/.cargo/bin:$PATH"` and `export RUSTUP_TOOLCHAIN=stable` in `build-all.sh`. CI uses rustup-managed toolchains so this is local-only.
2. **Wine in cross-rs windows-gnu image**: `wineboot` failed inside the container with `socket: Function not implemented` under the default Docker seccomp profile on Linux 7.x. Fix: `export CROSS_CONTAINER_OPTS="--security-opt seccomp=unconfined"`. Documented at the top of `build-all.sh`. Production CI alternative: GitHub Actions `windows-latest` runner builds natively.

**Implication for ROADMAP**:
- M1 release workflow (`.github/workflows/release.yml`) should use a **matrix of runners**, not Linux-only cross-compile:
  - `linux-x86_64`, `linux-aarch64`: Linux runner + `cross` (works as shown).
  - `windows-x86_64`: `windows-latest` runner + native `cargo build` (or Linux + `cross` with seccomp opt).
  - `macos-x86_64`, `macos-aarch64`: `macos-latest` runner + native `cargo build`.
- This is the standard `cargo-dist` / GitHub Actions release pattern. No architecture revision required.
- M0 also lets us drop one MVP scope-cut option: cross-platform binaries are cheap to ship at v0.1 if we accept multi-runner CI, so [ROADMAP.md scope-cut #1](../ROADMAP.md#what-gets-cut-if-mvp-scope-creeps) ("ship Linux + macOS only at v0.1") becomes unnecessary as long as macOS runners are available.

---

## Decisions for M1

1. **Proceed with §15 architecture as written** — no revisions needed.
2. **Release CI uses runner matrix, not Linux-only cross-compile.** Update §16.5 release recipe target list to be runner-aware.
3. **macOS hook-latency benchmark is part of M1 exit**, not deferred to M3. Without it, the M3 latency CI gate has only Linux numbers.
4. **Document the rustup-vs-system-rust trap and the Wine seccomp workaround** in `CONTRIBUTING.md` when M1 ships, so contributors hit them once at most.

---

## Artifacts

- `benchmarks/spike-m0/hook-latency/` — spike 1 source, bench scripts, raw results
  - `src/main.rs`, `Cargo.toml`, `bench.sh`, `bench-cold.sh`
  - `bench-results.txt`, `bench-cold-results.txt`
- `benchmarks/spike-m0/fastembed-parity/` — spike 2 Rust + Python embed, parity report
  - `src/main.rs`, `Cargo.toml`, `embed_python.py`, `compare.py`, `pyproject.toml`
  - `parity-report.txt`, `rust.stats.txt`, `python.stats.txt`
- `benchmarks/spike-m0/cross-compile/` — spike 3 build script + per-target logs
  - `build-all.sh`, `Cross.toml`, `Cargo.toml`, `src/`
  - `build-report.tsv`, `build-*.log`

Reproduce: `cd benchmarks/spike-m0/<spike>/ && <bench script>`. See each directory.
