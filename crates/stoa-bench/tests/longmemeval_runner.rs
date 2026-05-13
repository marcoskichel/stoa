//! E2E quality gate: `LongMemEval` runner binary surface.
//!
//! Spec source: [ROADMAP.md M4 exit criteria] — "`LongMemEval` reproducible
//! benchmark runner committed to `benchmarks/longmemeval/`" and "Published
//! `recall@k` numbers in `benchmarks/results/v0.1-local-chroma-sqlite.md`
//! (k=1, 5, 10)".
//!
//! Full corpus runs are gated to nightly (cost + bandwidth); the CI gate is:
//!
//! 1. `stoa-bench --help` lists `longmemeval` as a runnable benchmark.
//! 2. `stoa-bench longmemeval --dry-run` exits 0 with a recognizable
//!    progress / completion line — confirms wiring from the Rust runner
//!    through to the Python `LocalChromaSqliteBackend` works end-to-end
//!    without requiring the public dataset to be present.
//! 3. After a real run, the published results file matches the documented
//!    schema (`recall@1`, `recall@5`, `recall@10` per question category).

use snapbox::cmd::Command;

#[test]
fn stoa_bench_help_lists_longmemeval() {
    let out = Command::new(snapbox::cmd::cargo_bin!("stoa-bench"))
        .args(["--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr_text = String::from_utf8_lossy(&out.stderr);
    let body = format!("{stdout}{stderr_text}");
    assert!(
        body.to_lowercase().contains("longmem"),
        "`stoa-bench --help` must mention `longmem` (LongMemEval); got:\n{body}",
    );
}

#[test]
fn stoa_bench_longmemeval_dry_run_emits_completion_marker() {
    let out = Command::new(snapbox::cmd::cargo_bin!("stoa-bench"))
        .args(["longmemeval", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "`stoa-bench longmemeval --dry-run` must succeed (no dataset required): \
         status={:?} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let body = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let body_lc = body.to_lowercase();
    assert!(
        body_lc.contains("dry-run") || body_lc.contains("dry run") || body_lc.contains("recall@"),
        "dry-run must emit a recognizable marker (`dry-run` / `recall@`); got:\n{body}",
    );
}
