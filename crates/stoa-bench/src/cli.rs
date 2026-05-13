use std::path::PathBuf;

use clap::Parser;

/// Benchmark suite for Stoa's memory + recall pipeline.
#[derive(Debug, Parser)]
#[command(
    name = "stoa-bench",
    about = "Run the Stoa v0.1 benchmark suite against a recall backend"
)]
pub(crate) struct Cli {
    /// Recall backend to exercise.
    #[arg(long, value_enum, default_value_t = BackendKind::LocalChromaSqlite)]
    pub(crate) backend: BackendKind,

    /// Run a single benchmark; omit to run the full v0.1 suite.
    #[arg(long, value_enum)]
    pub(crate) bench: Option<BenchmarkKind>,

    /// Corpus cache root. Defaults to `benchmarks/corpus` relative to cwd.
    #[arg(long)]
    pub(crate) corpus_dir: Option<PathBuf>,

    /// Backbone LLM used for answer generation and judging.
    #[arg(long, default_value = "claude-haiku-4-5-20251001")]
    pub(crate) backbone_model: String,

    /// Pinned scorer git revision for result provenance.
    #[arg(long)]
    pub(crate) scorer_rev: Option<String>,

    /// Output directory for JSON result files. Prints to stdout when absent.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,

    /// Run a 5-question smoke slice to verify the pipeline wiring.
    #[arg(long)]
    pub(crate) smoke: bool,

    /// Stoa workspace root to point the recall backend at.
    ///
    /// When set, the bench uses `<workspace>/.stoa/queue.db` and
    /// `<workspace>/.stoa/recall.db` instead of a per-PID tempdir,
    /// so a running `stoa daemon` against the same workspace can
    /// serve vector recall requests via the Python sidecar.
    #[arg(long)]
    pub(crate) workspace: Option<PathBuf>,
}

/// Recall backend implementations.
#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum BackendKind {
    /// `ChromaDB` + `SQLite` FTS5 + KG — the default v0.1 backend.
    #[value(name = "local-chroma-sqlite")]
    LocalChromaSqlite,
    /// Zero-recall control arm for delta comparisons.
    #[value(name = "no-memory")]
    NoMemory,
}

impl BackendKind {
    /// Canonical string identifier used in result filenames.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::LocalChromaSqlite => "local-chroma-sqlite",
            Self::NoMemory => "no-memory",
        }
    }
}

/// v0.1 benchmark identifiers.
#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum BenchmarkKind {
    /// `LongMemEval` — multi-session recall + reasoning.
    #[value(name = "longmemeval")]
    Longmemeval,
    /// `MemoryAgentBench` — selective forgetting + test-time learning + retrieval.
    #[value(name = "memory-agent-bench")]
    MemoryAgentBench,
    /// `MEMTRACK` — multi-platform event-timeline state tracking.
    #[value(name = "memtrack")]
    Memtrack,
    /// `BEAM` — recall at 128K / 1M / 10M token scale.
    #[value(name = "beam")]
    Beam,
    /// `AgentLeak` — PII leak across 7 channel classes.
    #[value(name = "agent-leak")]
    AgentLeak,
    /// MTEB/BEIR subset — embedding component quality check.
    #[value(name = "mteb")]
    Mteb,
}
