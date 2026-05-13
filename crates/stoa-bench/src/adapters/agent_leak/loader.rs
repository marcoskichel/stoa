//! Corpus loader for the `AgentLeak` adapter.
//!
//! Two entry points: [`load_cases`] picks smoke vs. full based on
//! `RunParams::smoke`. The full path streams the upstream JSONL
//! line-by-line via [`BufReader`] so a 1000-scenario file never sits in
//! memory whole. Each scenario expands into 0+ [`Case`]s — one per
//! attacker target channel.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{
    adapter::{RunParams, load_smoke_fixture},
    error::BenchError,
};

/// Per-line size cap when streaming the full corpus — rejects hostile
/// records without blowing memory. The published scenarios are well under
/// 64 KiB each, so 1 MiB is a generous safety margin.
const MAX_LINE_BYTES: usize = 1 << 20;
/// Maximum tolerated malformed/over-long lines before the run is aborted.
const MAX_SKIPS: usize = 16;

/// One redactor-input case derived from an upstream `AgentLeak` scenario.
pub(crate) struct Case {
    pub(crate) channel: String,
    pub(crate) family: Option<String>,
    pub(crate) input: String,
    /// Literal PII strings the attacker is trying to exfiltrate. Sourced
    /// from the scenario's `evaluation.canaries` list when present,
    /// otherwise from `CANARY_*`-prefixed values in the first private
    /// vault record's `fields` map. Empty for smoke fixtures, which fall
    /// back to the difference-from-input proxy.
    pub(crate) canary_tokens: Vec<String>,
    pub(crate) expected_redacted: bool,
}

pub(crate) fn load_cases(params: &RunParams) -> Result<Vec<Case>, BenchError> {
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
        canary_tokens: Vec::new(),
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
    stream_scenarios(&path)
}

/// Stream the full corpus line-by-line and accumulate cases.
///
/// Malformed lines are skipped with a `tracing::warn!` so a single bad
/// record does not nuke the run; if skips exceed [`MAX_SKIPS`] the
/// loader fails loudly. Lines beyond [`MAX_LINE_BYTES`] are likewise
/// skipped (hostile-record guard).
fn stream_scenarios(path: &Path) -> Result<Vec<Case>, BenchError> {
    let reader = BufReader::new(File::open(path)?);
    let mut cases = Vec::new();
    let mut skipped = 0usize;
    for line in reader.lines() {
        let raw = line?;
        if let Some(extras) = process_line(&raw, &mut skipped) {
            cases.extend(extras);
        }
        if skipped > MAX_SKIPS {
            return Err(BenchError::CorpusParse(format!(
                "skip count exceeded {MAX_SKIPS}; corpus likely corrupt"
            )));
        }
    }
    Ok(cases)
}

fn process_line(raw: &str, skipped: &mut usize) -> Option<Vec<Case>> {
    if raw.len() > MAX_LINE_BYTES {
        *skipped += 1;
        tracing::warn!(target: "stoa_bench::agent_leak", bytes = raw.len(), "skipping over-long scenario line");
        return None;
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(scenario) => Some(cases_from_scenario(&scenario)),
        Err(err) => {
            *skipped += 1;
            tracing::warn!(target: "stoa_bench::agent_leak", error = %err, "skipping malformed scenario line");
            None
        },
    }
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
    let canary_tokens = scenario_canary_tokens(scenario);
    let snippet = vault_snippet(scenario);
    target_channels(attack)
        .into_iter()
        .map(|channel| Case {
            channel,
            family: family.clone(),
            input: assemble_input(&payload, &snippet),
            canary_tokens: canary_tokens.clone(),
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

/// Build the redactor input by concatenating attack payload + one
/// private-vault record. The snippet always contains at least one canary
/// / structured PII token so a working redactor has something to scrub.
fn assemble_input(payload: &str, snippet: &str) -> String {
    match (payload.is_empty(), snippet.is_empty()) {
        (true, true) => "[empty leak attempt]".to_owned(),
        (false, true) => payload.to_owned(),
        (true, false) => snippet.to_owned(),
        (false, false) => format!("{payload} :: {snippet}"),
    }
}

/// Extract the explicit canary string list for a scenario.
///
/// Prefers `evaluation.canaries` — the upstream-curated list of strings
/// the attacker is trying to exfiltrate (typically synthetic
/// `CANARY_<KIND>_<ID>` markers plus a small selection of vault PII
/// values). Falls back to vault-field-derived PII when the explicit
/// canary list is missing so the run still scores.
///
/// Using `evaluation.canaries` instead of the entire first vault record
/// avoids dragging in person names and address strings that the redactor
/// is not expected to scrub — they would otherwise be reported as
/// constant "leaks" and flatten the metric to zero.
fn scenario_canary_tokens(scenario: &Value) -> Vec<String> {
    let explicit = scenario
        .get("evaluation")
        .and_then(|v| v.get("canaries"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !explicit.is_empty() {
        return explicit;
    }
    vault_canary_fallback(scenario)
}

fn vault_canary_fallback(scenario: &Value) -> Vec<String> {
    let Some(fields) = first_vault_fields(scenario) else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|(_, v)| v.as_str().map(str::to_owned))
        .filter(|s| s.starts_with("CANARY_"))
        .collect()
}

fn first_vault_fields(scenario: &Value) -> Option<&serde_json::Map<String, Value>> {
    let records = scenario
        .get("private_vault")
        .and_then(|v| v.get("records"))
        .and_then(Value::as_array)?;
    let first = records.first()?;
    first.get("fields").and_then(Value::as_object)
}

/// Render the first vault record's string fields as `key=value` pairs
/// joined by ` | ` so canaries, SSNs, phones, and emails survive into the
/// redactor's input verbatim.
fn vault_snippet(scenario: &Value) -> String {
    let Some(fields) = first_vault_fields(scenario) else {
        return String::new();
    };
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in fields {
        if let Some(s) = v.as_str()
            && !s.is_empty()
        {
            parts.push(format!("{k}={s}"));
        }
    }
    parts.join(" | ")
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
