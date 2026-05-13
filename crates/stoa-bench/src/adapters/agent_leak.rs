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
//! NOTE: the published taxonomy ships F1–F4 across 6 attack classes
//! (`direct_prompt_injection`, `indirect_prompt_injection`, `role_confusion`,
//! `cross_agent_collusion`, `memory_write_exfiltration`,
//! `tool_output_poisoning`). F5/F6 are reserved by the paper but not yet
//! released in the corpus — they'll surface in the family breakout once
//! upstream publishes them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

/// `AgentLeak` adapter — channel + family PII leak probe.
///
/// Each input is run through `stoa-capture::Redactor::with_defaults()`. A case
/// is scored "correct" when the redacted output differs from input iff
/// `expected_redacted` is true. `missed_leaks` is the rate of cases that
/// *should* have been redacted but slipped through unchanged.
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
    family: Option<String>,
    input: String,
    expected_redacted: bool,
}

fn load_cases(params: &RunParams) -> Result<Vec<Case>, BenchError> {
    if params.smoke {
        return load_smoke_cases(&params.corpus_dir);
    }
    load_full_cases(&params.corpus_dir)
}

fn load_smoke_cases(corpus_dir: &Path) -> Result<Vec<Case>, BenchError> {
    let value = load_smoke_fixture(corpus_dir, "agent-leak")?;
    array_field(&value, "cases")?
        .iter()
        .map(parse_smoke_case)
        .collect()
}

fn parse_smoke_case(value: &Value) -> Result<Case, BenchError> {
    Ok(Case {
        channel: string_field(value, "channel")?,
        family: None,
        input: string_field(value, "input")?,
        expected_redacted: value
            .get("expected_redacted")
            .and_then(Value::as_bool)
            .ok_or_else(|| BenchError::CorpusParse("missing `expected_redacted`".to_owned()))?,
    })
}

fn load_full_cases(corpus_dir: &Path) -> Result<Vec<Case>, BenchError> {
    let path = full_corpus_path(corpus_dir);
    if !path.exists() {
        return Err(BenchError::CorpusMissing {
            path: path.to_string_lossy().into_owned(),
        });
    }
    let body = std::fs::read_to_string(&path)?;
    let mut cases = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let scenario: Value = serde_json::from_str(trimmed)?;
        cases.extend(cases_from_scenario(&scenario));
    }
    Ok(cases)
}

fn full_corpus_path(corpus_dir: &Path) -> PathBuf {
    corpus_dir.join("agent-leak/data/scenarios_full_1000.jsonl")
}

/// Convert one upstream scenario into 0+ `Case`s — one per attack target channel.
///
/// Only `attack.enabled == true` scenarios are emitted; the published corpus
/// also contains a benign control half (504/1000) with no attack metadata, and
/// those are skipped to keep `expected_redacted = true` invariant across the
/// reported cases.
fn cases_from_scenario(scenario: &Value) -> Vec<Case> {
    let Some(attack) = enabled_attack(scenario) else {
        return Vec::new();
    };
    let payload = attack_payload(attack);
    let family = attack_family(attack);
    let snippet = vault_snippet(scenario);
    target_channels(attack)
        .into_iter()
        .map(|channel| Case {
            channel,
            family: family.clone(),
            input: assemble_input(&payload, &snippet),
            expected_redacted: true,
        })
        .collect()
}

fn enabled_attack(scenario: &Value) -> Option<&Value> {
    let attack = scenario.get("attack")?;
    let enabled = attack
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    enabled.then_some(attack)
}

fn attack_payload(attack: &Value) -> String {
    attack
        .get("payload")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn attack_family(attack: &Value) -> Option<String> {
    attack
        .get("attack_family")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn target_channels(attack: &Value) -> Vec<String> {
    let raw = attack
        .get("target_channels")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone);
    let mut out: Vec<String> = raw
        .iter()
        .filter_map(Value::as_str)
        .map(map_channel_name)
        .collect();
    if out.is_empty() {
        out.push("C1".to_owned());
    }
    out
}

/// Map upstream channel string to the C1–C7 code per project DOCUMENTATION.md.
fn map_channel_name(raw: &str) -> String {
    match raw {
        "final_output" => "C1",
        "inter_agent" => "C2",
        "tool_input" => "C3",
        "tool_output" => "C4",
        "memory_write" | "memory" => "C5",
        "log" | "logs" => "C6",
        "artifact" | "artifacts" => "C7",
        other => other,
    }
    .to_owned()
}

/// Build the redactor input by concatenating attack payload + one private-vault
/// record. The snippet always contains at least one canary / structured PII
/// token so a working redactor has something to scrub.
fn assemble_input(payload: &str, snippet: &str) -> String {
    match (payload.is_empty(), snippet.is_empty()) {
        (true, true) => "[empty leak attempt]".to_owned(),
        (false, true) => payload.to_owned(),
        (true, false) => snippet.to_owned(),
        (false, false) => format!("{payload} :: {snippet}"),
    }
}

/// Flatten the first private-vault record's fields into a single line.
///
/// Each record under `private_vault.records[*].fields` is a flat string→value
/// map. We pick the first record and serialise its leaf string fields as
/// `key=value` pairs separated by ` | ` so canaries, SSNs, phones, and emails
/// survive into the redactor's input verbatim.
fn vault_snippet(scenario: &Value) -> String {
    let Some(records) = scenario
        .get("private_vault")
        .and_then(|v| v.get("records"))
        .and_then(Value::as_array)
    else {
        return String::new();
    };
    let first = records.iter().next();
    let fields = first
        .and_then(|r| r.get("fields"))
        .and_then(Value::as_object);
    let Some(fields) = fields else {
        return String::new();
    };
    fields
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| format!("{k}={s}")))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn score_cases(cases: &[Case]) -> BTreeMap<String, f64> {
    let redactor = Redactor::with_defaults();
    let mut tally = Tally::default();
    for case in cases {
        let redacted = redactor.redact_line(&case.input);
        let was_redacted = redacted != case.input;
        let correct = was_redacted == case.expected_redacted;
        let missed = case.expected_redacted && !was_redacted;
        tally.record(&case.channel, case.family.as_deref(), correct, missed);
    }
    finalize(&tally, cases.len())
}

#[derive(Default)]
struct Bucket {
    total: usize,
    correct: usize,
    missed_leaks: usize,
}

impl Bucket {
    fn observe(&mut self, correct: bool, missed: bool) {
        self.total += 1;
        if correct {
            self.correct += 1;
        }
        if missed {
            self.missed_leaks += 1;
        }
    }
}

#[derive(Default)]
struct Tally {
    by_channel: BTreeMap<String, Bucket>,
    by_family: BTreeMap<String, Bucket>,
}

impl Tally {
    fn record(&mut self, channel: &str, family: Option<&str>, correct: bool, missed: bool) {
        self.by_channel
            .entry(channel.to_owned())
            .or_default()
            .observe(correct, missed);
        let fam_key = family.unwrap_or("F0").to_owned();
        self.by_family
            .entry(fam_key)
            .or_default()
            .observe(correct, missed);
    }
}

fn finalize(tally: &Tally, total_cases: usize) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let (overall_correct, overall_missed) =
        aggregate(&tally.by_channel, "accuracy", "missed_leaks", &mut out);
    aggregate(&tally.by_family, "accuracy", "missed_leaks", &mut out);
    out.insert("accuracy".to_owned(), safe_div(overall_correct, total_cases));
    out.insert("missed_leaks".to_owned(), safe_div(overall_missed, total_cases));
    out
}

fn aggregate(
    buckets: &BTreeMap<String, Bucket>,
    acc_prefix: &str,
    missed_prefix: &str,
    out: &mut BTreeMap<String, f64>,
) -> (usize, usize) {
    let mut correct = 0usize;
    let mut missed = 0usize;
    for (label, b) in buckets {
        out.insert(format!("{acc_prefix}:{label}"), safe_div(b.correct, b.total));
        out.insert(format!("{missed_prefix}:{label}"), safe_div(b.missed_leaks, b.total));
        correct += b.correct;
        missed += b.missed_leaks;
    }
    (correct, missed)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "case counts bounded by corpus size; well under 2^52"
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
        corpus_rev: corpus_rev_label(params.smoke),
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

fn corpus_rev_label(smoke: bool) -> String {
    if smoke {
        "smoke".to_owned()
    } else {
        "github-1.0.0".to_owned()
    }
}
