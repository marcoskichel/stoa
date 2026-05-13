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
    let md = render(result)?;
    let filename = format!("{}-{}-{}.md", result.version, result.backend, result.benchmark);
    std::fs::write(dir.join(filename), md)?;
    Ok(())
}

fn render(r: &BenchmarkResult) -> Result<String, BenchError> {
    let mut buf = String::new();
    write_header(&mut buf, r)?;
    write_metrics(&mut buf, r)?;
    write_hyperparams(&mut buf, r)?;
    write_provenance(&mut buf, r)?;
    Ok(buf)
}

fn write_header(buf: &mut String, r: &BenchmarkResult) -> Result<(), BenchError> {
    writeln!(buf, "# {} — {} v{}\n", r.benchmark, r.backend, r.version).map_err(fmt_err)?;
    writeln!(
        buf,
        "**Run:** {}  **Wall:** {}s  **Cost:** ${:.4}\n",
        r.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
        r.wall_seconds,
        r.cost_usd,
    )
    .map_err(fmt_err)
}

fn write_metrics(buf: &mut String, r: &BenchmarkResult) -> Result<(), BenchError> {
    writeln!(buf, "## Metrics\n").map_err(fmt_err)?;
    writeln!(buf, "| Metric | Value |").map_err(fmt_err)?;
    writeln!(buf, "|--------|-------|").map_err(fmt_err)?;
    for (k, v) in &r.metrics {
        writeln!(buf, "| {k} | {v:.3} |").map_err(fmt_err)?;
    }
    writeln!(buf).map_err(fmt_err)
}

fn write_hyperparams(buf: &mut String, r: &BenchmarkResult) -> Result<(), BenchError> {
    writeln!(buf, "## Hyperparameters\n").map_err(fmt_err)?;
    writeln!(buf, "| Key | Value |").map_err(fmt_err)?;
    writeln!(buf, "|-----|-------|").map_err(fmt_err)?;
    for (k, v) in &r.hyperparams {
        writeln!(buf, "| {k} | {v} |").map_err(fmt_err)?;
    }
    writeln!(buf).map_err(fmt_err)
}

fn write_provenance(buf: &mut String, r: &BenchmarkResult) -> Result<(), BenchError> {
    writeln!(buf, "## Provenance\n").map_err(fmt_err)?;
    writeln!(buf, "- **Corpus revision:** {}", r.corpus_rev).map_err(fmt_err)?;
    writeln!(buf, "- **Scorer revision:** {}", r.scorer_rev).map_err(fmt_err)?;
    writeln!(buf, "- **Backbone model:** {}", r.backbone_model).map_err(fmt_err)
}

fn fmt_err(e: std::fmt::Error) -> BenchError {
    BenchError::Io(std::io::Error::other(e.to_string()))
}
