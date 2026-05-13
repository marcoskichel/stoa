//! `AgentLeakAdapter` — PII redaction probe across the 7-channel taxonomy.
//!
//! Scoring buckets:
//! - per channel (`accuracy:C1`..`C7`, `missed_leaks:C1`..`C7`)
//! - per attack family (`accuracy:F1`..`F6`, `missed_leaks:F1`..`F6`)
//!
//! Cases come from one of two sources:
//! - `--smoke` → `benchmarks/agent-leak/fixtures/smoke.json` (committed).
//! - default → `benchmarks/corpus/agent-leak/data/scenarios_full_1000.jsonl`
//!   downloaded by `benchmarks/corpus/agent-leak.sh` from the
//!   `Privatris/AgentLeak` GitHub repo. The `HuggingFace` mirror
//!   (`humain2/AgentLeak`) currently only carries a README.
//!
//! Scoring methodology: for each enabled-attack scenario the adapter
//! pulls the upstream-curated list at `evaluation.canaries` (falling
//! back to `CANARY_*`-prefixed values inside the first vault record's
//! `fields` map when the explicit list is missing) as the canary tokens
//! the attacker is trying to exfiltrate. After redaction, a case is
//! correct iff none of those tokens survive verbatim in the output. A
//! surviving token counts as a missed leak — the metric the
//! `AgentLeak` benchmark was designed to measure.
//!
//! NOTE: the published taxonomy ships F1–F4 across 6 attack classes
//! (`direct_prompt_injection`, `indirect_prompt_injection`, `role_confusion`,
//! `cross_agent_collusion`, `memory_write_exfiltration`,
//! `tool_output_poisoning`). F5/F6 are reserved by the paper but not yet
//! released in the corpus — they'll surface in the family breakout once
//! upstream publishes them.

mod loader;
mod scoring;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

pub(crate) use loader::Case;

/// `AgentLeak` adapter — channel + family PII leak probe.
///
/// Each input is run through `stoa-capture::Redactor::with_defaults()`.
/// A case is scored "correct" when no canary token from the scenario's
/// `evaluation.canaries` list survives verbatim in the redacted output
/// (when `expected_redacted` is true) or, for benign controls, when the
/// redactor leaves the input untouched.
pub(crate) struct AgentLeakAdapter;

#[async_trait]
impl BenchmarkAdapter for AgentLeakAdapter {
    fn name(&self) -> &'static str {
        "agent-leak"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let cases = loader::load_cases(params)?;
        let started = Instant::now();
        let metrics = scoring::score_cases(&cases);
        Ok(build_result(self.name(), params, metrics, started))
    }
}

fn build_result(
    name: &'static str,
    params: &RunParams,
    metrics: BTreeMap<String, f64>,
    started: Instant,
) -> BenchmarkResult {
    BenchmarkResult {
        benchmark: name.to_owned(),
        backend: params.backend_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        corpus_rev: corpus_rev_label(params.smoke, &params.corpus_dir),
        scorer_rev: params.scorer_rev.clone(),
        backbone_model: params.backbone_model.clone(),
        hyperparams: BTreeMap::from([(
            "redactor".to_owned(),
            Value::String("stoa-capture::Redactor::with_defaults".to_owned()),
        )]),
        metrics,
        cost_usd: 0.0,
        tokens_used: 0,
        wall_seconds: started.elapsed().as_secs(),
        timestamp: Utc::now(),
    }
}

/// Build the corpus revision label written into the result JSON.
///
/// In smoke mode the label is the literal `"smoke"`. For the full corpus,
/// the label is read from `<corpus>/agent-leak/.version` (written by
/// `benchmarks/corpus/agent-leak.sh` with the short SHA). When the file
/// is missing, falls back to `"gh-unpinned"` so the run still completes
/// but the provenance is clearly tagged as un-resolved.
fn corpus_rev_label(smoke: bool, corpus_dir: &Path) -> String {
    if smoke {
        return "smoke".to_owned();
    }
    let version_path = corpus_dir.join("agent-leak/.version");
    std::fs::read_to_string(&version_path)
        .map_or_else(|_| "gh-unpinned".to_owned(), |s| s.trim().to_owned())
}
