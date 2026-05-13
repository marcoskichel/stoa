//! `LongMemEval` runner. Per ROADMAP M4 exit criteria:
//!
//! - `stoa-bench longmemeval --dry-run` exits 0 with completion marker
//!   (CI-safe, no dataset required).
//! - Real run reads `benchmarks/longmemeval/data/` (gitignored), invokes
//!   the Python `LocalChromaSqliteBackend.search` over each query, writes
//!   ranked top-K, scores per the upstream evaluator (Wang et al. ICLR
//!   2025; pin `gpt-4o-2024-11-20`).
//!
//! Scoring + dataset download live in the Python sidecar (`stoa-bench`
//! Rust side is just the orchestrator); the dataset is `HuggingFace`
//! `xiaowu0162/longmemeval-cleaned`.

use std::path::PathBuf;

use clap::Args as ClapArgs;

/// Arguments for `stoa-bench longmemeval`.
#[derive(ClapArgs, Debug)]
pub(crate) struct Args {
    /// Skip the dataset; print completion marker + placeholder metrics.
    #[arg(long)]
    pub dry_run: bool,

    /// Path to the `LongMemEval` dataset root. Defaults to
    /// `benchmarks/longmemeval/data/`.
    #[arg(long)]
    pub data_path: Option<PathBuf>,

    /// `recall@k` cap. Defaults to 10.
    #[arg(long, default_value_t = 10)]
    pub k: usize,
}

/// Run the `LongMemEval` benchmark.
pub(crate) fn run(args: &Args) -> anyhow::Result<()> {
    if args.dry_run {
        emit_dry_run();
        return Ok(());
    }
    run_real(args)
}

#[expect(
    clippy::print_stdout,
    reason = "Benchmark CLI emits progress + summary to stdout by design."
)]
fn emit_dry_run() {
    println!("dry-run complete");
    println!("recall@1=N/A");
    println!("recall@5=N/A");
    println!("recall@10=N/A");
}

#[expect(
    clippy::print_stdout,
    reason = "Benchmark CLI emits progress + summary to stdout by design."
)]
fn run_real(args: &Args) -> anyhow::Result<()> {
    let data = args
        .data_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("benchmarks/longmemeval/data/"));
    if !data.is_dir() {
        return Err(anyhow::anyhow!(
            "`LongMemEval` dataset not found at `{}`; download from \
             huggingface `xiaowu0162/longmemeval-cleaned` (see benchmarks/longmemeval/README.md)",
            data.display(),
        ));
    }
    println!("LongMemEval real run not implemented in M4 (--dry-run only).");
    println!("Dataset path: {}", data.display());
    println!("recall@{}=pending", args.k);
    Ok(())
}
