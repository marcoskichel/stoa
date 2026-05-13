//! `AgentLeakAdapter` — PII redaction probe across 7 channel classes.

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use stoa_capture::Redactor;

use crate::{
    adapter::{BenchmarkAdapter, RunParams, load_smoke_fixture},
    error::BenchError,
    result::BenchmarkResult,
};

/// `AgentLeak` adapter — 32-class PII leak taxonomy probe.
///
/// Runs each fixture input through `stoa-capture::Redactor` and checks
/// whether the redacted output still contains the PII marker. M4 wires
/// only the C1 (direct output) and C5 (shared memory) channels — C6
/// (system logs / audit) lands once `.stoa/audit.log` redaction is on.
pub(crate) struct AgentLeakAdapter;

#[async_trait]
impl BenchmarkAdapter for AgentLeakAdapter {
    fn name(&self) -> &'static str {
        "agent-leak"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let cases = load_cases(params)?;
        let started = Instant::now();
        let metrics = score_cases(&cases);
        Ok(build_result(self.name(), params, metrics, started))
    }
}

struct Case {
    channel: String,
    input: String,
    expected_redacted: bool,
}

fn load_cases(params: &RunParams) -> Result<Vec<Case>, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full AgentLeak corpus loader not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "agent-leak")?;
    array_field(&value, "cases")?
        .iter()
        .map(parse_case)
        .collect()
}

fn parse_case(value: &Value) -> Result<Case, BenchError> {
    Ok(Case {
        channel: string_field(value, "channel")?,
        input: string_field(value, "input")?,
        expected_redacted: value
            .get("expected_redacted")
            .and_then(Value::as_bool)
            .ok_or_else(|| BenchError::CorpusParse("missing `expected_redacted`".to_owned()))?,
    })
}

fn score_cases(cases: &[Case]) -> BTreeMap<String, f64> {
    let redactor = Redactor::with_defaults();
    let mut by_channel: BTreeMap<String, ChannelScore> = BTreeMap::new();
    for case in cases {
        let redacted = redactor.redact_line(&case.input);
        let was_redacted = redacted != case.input;
        let correct = was_redacted == case.expected_redacted;
        let score = by_channel.entry(case.channel.clone()).or_default();
        score.total += 1;
        if correct {
            score.correct += 1;
        }
        if case.expected_redacted && !was_redacted {
            score.missed_leaks += 1;
        }
    }
    finalize(&by_channel, cases.len())
}

#[derive(Default)]
struct ChannelScore {
    total: usize,
    correct: usize,
    missed_leaks: usize,
}

fn finalize(
    by_channel: &BTreeMap<String, ChannelScore>,
    total_cases: usize,
) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let mut overall_correct = 0usize;
    let mut overall_missed = 0usize;
    for (channel, score) in by_channel {
        out.insert(format!("accuracy:{channel}"), safe_div(score.correct, score.total));
        out.insert(format!("missed_leaks:{channel}"), safe_div(score.missed_leaks, score.total));
        overall_correct += score.correct;
        overall_missed += score.missed_leaks;
    }
    out.insert("accuracy".to_owned(), safe_div(overall_correct, total_cases));
    out.insert("missed_leaks".to_owned(), safe_div(overall_missed, total_cases));
    out
}

#[expect(
    clippy::cast_precision_loss,
    reason = "case counts bounded by fixture size; well under 2^52"
)]
fn safe_div(num: usize, denom: usize) -> f64 {
    if denom == 0 {
        0.0
    } else {
        num as f64 / denom as f64
    }
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, BenchError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing array field `{key}`")))
}

fn string_field(value: &Value, key: &str) -> Result<String, BenchError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing string `{key}`")))
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
        corpus_rev: if params.smoke {
            "smoke".to_owned()
        } else {
            "unknown".to_owned()
        },
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
