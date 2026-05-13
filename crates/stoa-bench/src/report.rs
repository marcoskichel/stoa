//! Markdown report writer for benchmark results.
//!
//! Produces a human-readable `.md` file alongside the machine-readable `.json`
//! so CI artifacts and PR comments can surface results without requiring a
//! JSON parser.

use std::fmt::Write as _;
use std::path::Path;

use crate::{error::BenchError, result::BenchmarkResult};

/// Write a markdown summary of `result` into `dir`.
///
/// The filename mirrors the JSON counterpart:
/// `<version>-<backend>-<benchmark>.md`. Overwrites any existing file at
/// that path.
pub(crate) fn write_markdown(result: &BenchmarkResult, dir: &Path) -> Result<(), BenchError> {
    let md = render(result);
    let stem = crate::run::result_filename_stem(result);
    std::fs::write(dir.join(format!("{stem}.md")), md)?;
    Ok(())
}

/// Render `r` into a self-contained markdown string.
///
/// `write!` to a `String` is infallible — `std::fmt::Write::write_fmt` only
/// fails on allocator OOM, which panics rather than returning `Err`. We
/// discard the `Result` to keep the call path off the bogus
/// `std::fmt::Error → io::Error` mapping that the previous version used.
fn render(r: &BenchmarkResult) -> String {
    let mut buf = String::new();
    push_header(&mut buf, r);
    push_metrics(&mut buf, r);
    push_hyperparams(&mut buf, r);
    push_provenance(&mut buf, r);
    buf
}

fn push_header(buf: &mut String, r: &BenchmarkResult) {
    let _ = writeln!(buf, "# {} — {} v{}\n", r.benchmark, r.backend, r.version);
    let _ = writeln!(
        buf,
        "**Run:** {}  **Wall:** {}s  **Cost:** ${:.4}\n",
        r.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
        r.wall_seconds,
        r.cost_usd,
    );
}

fn push_metrics(buf: &mut String, r: &BenchmarkResult) {
    let _ = writeln!(buf, "## Metrics\n");
    let _ = writeln!(buf, "| Metric | Value |");
    let _ = writeln!(buf, "|--------|-------|");
    for (k, v) in &r.metrics {
        let _ = writeln!(buf, "| {k} | {v:.3} |");
    }
    buf.push('\n');
}

fn push_hyperparams(buf: &mut String, r: &BenchmarkResult) {
    let _ = writeln!(buf, "## Hyperparameters\n");
    let _ = writeln!(buf, "| Key | Value |");
    let _ = writeln!(buf, "|-----|-------|");
    for (k, v) in &r.hyperparams {
        let _ = writeln!(buf, "| {k} | {v} |");
    }
    buf.push('\n');
}

fn push_provenance(buf: &mut String, r: &BenchmarkResult) {
    let _ = writeln!(buf, "## Provenance\n");
    let _ = writeln!(buf, "- **Corpus revision:** {}", r.corpus_rev);
    let _ = writeln!(buf, "- **Scorer revision:** {}", r.scorer_rev);
    let _ = writeln!(buf, "- **Backbone model:** {}", r.backbone_model);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use assert_fs::TempDir;
    use chrono::TimeZone;

    use super::*;

    fn sample_result() -> BenchmarkResult {
        let metrics: BTreeMap<String, f64> = [("recall@10".to_owned(), 0.123)].into();
        let hyperparams: BTreeMap<String, serde_json::Value> =
            [("k".to_owned(), serde_json::json!(10))].into();
        BenchmarkResult {
            benchmark: "longmemeval".to_owned(),
            backend: "test-backend".to_owned(),
            version: "0.1.0".to_owned(),
            corpus_rev: "corpus-abc".to_owned(),
            scorer_rev: "scorer-def".to_owned(),
            backbone_model: "test-model".to_owned(),
            hyperparams,
            metrics,
            cost_usd: 0.0,
            tokens_used: 0,
            wall_seconds: 1,
            timestamp: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn render_produces_expected_sections() {
        let result = sample_result();
        let md = render(&result);
        assert!(md.contains("# longmemeval"));
        assert!(md.contains("| recall@10 | 0.123 |"));
        assert!(md.contains("**Backbone model:** test-model"));
    }

    #[test]
    fn write_markdown_creates_file_in_dir() {
        let tmp = TempDir::new().unwrap();
        let result = sample_result();
        write_markdown(&result, tmp.path()).unwrap();
        let expected = tmp.path().join("v0.1-test-backend-longmemeval.md");
        assert!(expected.exists());
    }
}
