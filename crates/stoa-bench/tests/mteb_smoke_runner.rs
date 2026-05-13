//! E2E quality gate: `stoa-bench --bench mteb --smoke` runner surface.
//!
//! Pre-corpus CI gate: verify the binary exits cleanly and writes both the
//! JSON result and Markdown summary for the MTEB smoke fixture committed
//! under `benchmarks/mteb-retrieval/fixtures/`.

use std::path::{Path, PathBuf};
use std::process::Output;

use snapbox::cmd::Command;

fn has_extension(name: &str, ext: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

#[test]
fn stoa_bench_help_lists_mteb() {
    let body = collect_help_output().unwrap();
    assert!(
        body.to_lowercase().contains("mteb"),
        "`stoa-bench --help` must mention `mteb`; got:\n{body}",
    );
}

#[test]
fn stoa_bench_mteb_smoke_emits_result_files() {
    let tmp = tempdir().unwrap();
    let tmp_str = tmp.to_str().unwrap();
    let out = run_smoke(tmp_str).unwrap();
    assert!(out.status.success(), "mteb smoke run failed: {out:?}");
    let written = list_dir(&tmp).unwrap();
    assert!(
        written
            .iter()
            .any(|f| f.contains("mteb") && has_extension(f, "json")),
        "expected an mteb .json result file, found: {written:?}",
    );
    assert!(
        written
            .iter()
            .any(|f| f.contains("mteb") && has_extension(f, "md")),
        "expected an mteb .md result file, found: {written:?}",
    );
}

fn collect_help_output() -> std::io::Result<String> {
    let out = Command::new(snapbox::cmd::cargo_bin!("stoa-bench"))
        .args(["--help"])
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr_text = String::from_utf8_lossy(&out.stderr);
    Ok(format!("{stdout}{stderr_text}"))
}

fn run_smoke(out_dir: &str) -> std::io::Result<Output> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let corpus = format!("{manifest}/../../benchmarks/corpus");
    Command::new(snapbox::cmd::cargo_bin!("stoa-bench"))
        .args([
            "--bench",
            "mteb",
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
    dir.push(format!("stoa-bench-mteb-test-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
