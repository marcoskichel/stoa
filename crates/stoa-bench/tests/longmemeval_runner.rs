//! E2E quality gate: `stoa-bench` runner binary surface.
//!
//! CI gate (pre-corpus): `stoa-bench --bench longmemeval --smoke --output <dir>`
//! exits 0 and writes a JSON result file with the canonical
//! `<version>-<backend>-<benchmark>.json` filename.

use std::path::{Path, PathBuf};
use std::process::Output;

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
fn stoa_bench_longmemeval_smoke_emits_result_file() {
    let tmp = tempdir().unwrap();
    let tmp_str = tmp.to_str().unwrap();
    let out = run_smoke(tmp_str).unwrap();
    assert!(out.status.success(), "smoke run failed: {out:?}");
    let written = list_dir(&tmp).unwrap();
    assert!(
        written.iter().any(|f| f.contains("longmemeval")),
        "expected a longmemeval result file, found: {written:?}",
    );
    assert!(
        written
            .iter()
            .any(|f| Path::new(f).extension().is_some_and(|e| e == "md")
                && f.contains("longmemeval")),
        "expected longmemeval .md file, found: {written:?}",
    );
}

fn run_smoke(out_dir: &str) -> std::io::Result<Output> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let corpus = format!("{manifest}/../../benchmarks/corpus");
    Command::new(snapbox::cmd::cargo_bin!("stoa-bench"))
        .args([
            "--bench",
            "longmemeval",
            "--smoke",
            "--corpus-dir",
            &corpus,
            "--output",
            out_dir,
        ])
        .output()
}

fn list_dir(dir: &Path) -> std::io::Result<Vec<String>> {
    let entries = std::fs::read_dir(dir)?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect())
}

fn tempdir() -> std::io::Result<PathBuf> {
    let mut dir = std::env::temp_dir();
    dir.push(format!("stoa-bench-test-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
