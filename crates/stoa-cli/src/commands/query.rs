//! `stoa query` — daemon-driven semantic + BM25 search.

use anyhow::{Result, anyhow};
use stoa_recall::{Filters, MempalaceBackend, RecallBackend};

use crate::cli::QueryArgs;

/// Run `stoa query`.
pub(crate) async fn run(args: QueryArgs) -> Result<()> {
    let backend = MempalaceBackend::from_env();
    let filters = pick_filters(args.include_drawers);
    let hits = backend
        .search(&args.query, args.top_k, &filters)
        .await
        .map_err(|e| anyhow!("daemon search failed: {e}"))?;
    if hits.is_empty() {
        println("No hits.");
        return Ok(());
    }
    print_hits(&hits);
    Ok(())
}

fn pick_filters(include_drawers: bool) -> Filters {
    if include_drawers {
        Filters::default()
    } else {
        Filters::wiki_only()
    }
}

fn print_hits(hits: &[stoa_recall::Hit]) {
    for (i, h) in hits.iter().enumerate() {
        println(&format!(
            "{rank}. [score={score:.3}] {path}\n   {snippet}",
            rank = i + 1,
            score = h.score,
            path = h.source_path.as_str(),
            snippet = h.snippet.trim().lines().next().unwrap_or(""),
        ));
    }
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
