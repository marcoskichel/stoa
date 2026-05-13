//! `stoa write` — write a wiki page through the daemon.

use std::fs;

use anyhow::{Context, Result, anyhow};
use stoa_recall::{MempalaceBackend, RecallBackend};

use crate::cli::WriteArgs;

/// Run `stoa write`.
pub(crate) async fn run(args: WriteArgs) -> Result<()> {
    let raw_fm = fs::read_to_string(&args.frontmatter)
        .with_context(|| format!("reading {}", args.frontmatter.display()))?;
    let body = fs::read_to_string(&args.body)
        .with_context(|| format!("reading {}", args.body.display()))?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&raw_fm).context("parsing frontmatter YAML")?;
    let json_fm: serde_json::Value =
        serde_json::to_value(&yaml).context("converting frontmatter YAML → JSON for the wire")?;
    let backend = MempalaceBackend::from_env();
    let path = backend
        .write_wiki(&args.page_id, &json_fm, &body)
        .await
        .map_err(|e| anyhow!("daemon write_wiki failed: {e}"))?;
    println(&format!("Wrote {} ({})", args.page_id, path));
    Ok(())
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
