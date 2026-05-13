//! `MemtrackAdapter` — multi-platform event-timeline state tracking.

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use stoa_recall::{Filters, StreamSet};

use crate::{
    adapter::{BenchmarkAdapter, RunParams, load_smoke_fixture},
    error::BenchError,
    result::BenchmarkResult,
};

const MAX_K: usize = 10;

/// `MEMTRACK` — 47 expert scenarios across Slack/Linear/Gitea.
///
/// Smoke uses exact-substring scoring instead of the LLM judge from the
/// paper (no public scorer; judge implementation lands as a follow-up).
pub(crate) struct MemtrackAdapter;

#[async_trait]
impl BenchmarkAdapter for MemtrackAdapter {
    fn name(&self) -> &'static str {
        "memtrack"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let instances = load_instances(params)?;
        let started = Instant::now();
        let metrics = score_instances(&instances, params).await?;
        Ok(build_result(self.name(), params, metrics, started))
    }
}

struct Instance {
    id: String,
    events: Vec<EventRow>,
    qa_pairs: Vec<QaPair>,
}

struct EventRow {
    body: String,
}

struct QaPair {
    question: String,
    answer: String,
}

fn load_instances(params: &RunParams) -> Result<Vec<Instance>, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full MEMTRACK corpus loader not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "memtrack")?;
    array_field(&value, "instances")?
        .iter()
        .map(parse_instance)
        .collect()
}

fn parse_instance(value: &Value) -> Result<Instance, BenchError> {
    Ok(Instance {
        id: string_field(value, "instance_id")?,
        events: parse_events(value)?,
        qa_pairs: parse_qa_pairs(value)?,
    })
}

fn parse_events(value: &Value) -> Result<Vec<EventRow>, BenchError> {
    Ok(array_field(value, "timeline")?
        .iter()
        .map(flatten_event)
        .collect())
}

/// Flatten an event's heterogeneous platform fields into a single
/// searchable blob. Joins all string values; numbers + arrays are
/// stringified.
fn flatten_event(value: &Value) -> EventRow {
    let mut parts = Vec::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            parts.push(format!("{k}={}", value_to_string(v)));
        }
    }
    EventRow { body: parts.join(" ") }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn parse_qa_pairs(value: &Value) -> Result<Vec<QaPair>, BenchError> {
    array_field(value, "qa_pairs")?
        .iter()
        .map(|v| {
            Ok(QaPair {
                question: string_field(v, "question")?,
                answer: string_field(v, "answer")?,
            })
        })
        .collect()
}

async fn score_instances(
    instances: &[Instance],
    params: &RunParams,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut hits = 0.0;
    let mut total = 0usize;
    for inst in instances {
        index_events(inst, params).await?;
        for qa in &inst.qa_pairs {
            total += 1;
            hits += score_one(qa, params).await?;
        }
    }
    Ok(finalize(hits, total))
}

async fn index_events(inst: &Instance, params: &RunParams) -> Result<(), BenchError> {
    for (i, event) in inst.events.iter().enumerate() {
        let id = format!("{}:e{i}", inst.id);
        let path = format!("instances/{}.jsonl", inst.id);
        params
            .backend
            .index_page(&id, &event.body, &path, &Value::Null)
            .await?;
    }
    Ok(())
}

async fn score_one(qa: &QaPair, params: &RunParams) -> Result<f64, BenchError> {
    let hits = params
        .backend
        .search(&qa.question, MAX_K, &Filters::default(), StreamSet::all())
        .await?;
    let blob = hits
        .iter()
        .map(|h| h.snippet.clone())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    Ok(if blob.contains(&qa.answer.to_lowercase()) {
        1.0
    } else {
        0.0
    })
}

fn finalize(hits: f64, n: usize) -> BTreeMap<String, f64> {
    if n == 0 {
        return BTreeMap::new();
    }
    BTreeMap::from([("correctness".to_owned(), hits / safe_div(n))])
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, BenchError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing array `{key}`")))
}

fn string_field(value: &Value, key: &str) -> Result<String, BenchError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing string `{key}`")))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "QA counts bounded by 47 instances × 5 Q max"
)]
fn safe_div(n: usize) -> f64 {
    n as f64
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
        hyperparams: BTreeMap::from([
            ("k".to_owned(), Value::Number(serde_json::Number::from(MAX_K))),
            ("scoring".to_owned(), Value::String("substring".to_owned())),
        ]),
        metrics,
        cost_usd: 0.0,
        tokens_used: 0,
        wall_seconds: started.elapsed().as_secs(),
        timestamp: Utc::now(),
    }
}
