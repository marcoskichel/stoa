//! `stoa read` — pull a wiki page back from the daemon.

use anyhow::{Result, anyhow};
use stoa_recall::{MempalaceBackend, RecallBackend};

use crate::cli::ReadArgs;

/// Run `stoa read`.
pub(crate) async fn run(args: ReadArgs) -> Result<()> {
    let backend = MempalaceBackend::from_env();
    let (frontmatter, body) = backend
        .read_wiki(&args.page_id)
        .await
        .map_err(|e| anyhow!("daemon read_wiki failed: {e}"))?;
    let fm_yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|e| anyhow!("rendering frontmatter as YAML: {e}"))?;
    println(&format!("---\n{fm_yaml}---\n\n{body}"));
    Ok(())
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
