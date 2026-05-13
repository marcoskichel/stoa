//! `MemoryAgentBenchAdapter` — 4 top-level splits + per-split scoring.

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
const SPLIT_NAMES: [&str; 4] = [
    "Accurate_Retrieval",
    "Test_Time_Learning",
    "Long_Range_Understanding",
    "Conflict_Resolution",
];

/// `MemoryAgentBench` adapter — four-competency agentic memory probe.
pub(crate) struct MemoryAgentBenchAdapter;

#[async_trait]
impl BenchmarkAdapter for MemoryAgentBenchAdapter {
    fn name(&self) -> &'static str {
        "memory-agent-bench"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let dataset = load_dataset(params)?;
        let started = Instant::now();
        let metrics = score_dataset(&dataset, params).await?;
        Ok(build_result(self.name(), params, metrics, started))
    }
}

struct Task {
    split: String,
    context: String,
    questions: Vec<String>,
    answers: Vec<String>,
    doc_id: String,
}

fn load_dataset(params: &RunParams) -> Result<Vec<Task>, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full MemoryAgentBench corpus loader not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "memory-agent-bench")?;
    let mut out = Vec::new();
    for split in SPLIT_NAMES {
        let entries = value
            .get(split)
            .and_then(Value::as_array)
            .ok_or_else(|| BenchError::CorpusParse(format!("missing split `{split}`")))?;
        for (idx, entry) in entries.iter().enumerate() {
            out.push(parse_task(split, idx, entry)?);
        }
    }
    Ok(out)
}

fn parse_task(split: &str, idx: usize, value: &Value) -> Result<Task, BenchError> {
    let context = value
        .get("context")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BenchError::CorpusParse(format!("{split}[{idx}].context missing")))?;
    let questions = string_array(value, "questions").unwrap_or_default();
    let answers = string_array(value, "answers").unwrap_or_default();
    let task_id = value
        .get("metadata")
        .and_then(|m| m.get("qa_pair_ids"))
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .map_or_else(|| format!("{split}-{idx}"), str::to_owned);
    Ok(Task {
        split: split.to_owned(),
        context,
        questions,
        answers,
        doc_id: task_id,
    })
}

fn string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key).and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    })
}

async fn score_dataset(
    tasks: &[Task],
    params: &RunParams,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut by_split: BTreeMap<String, SplitScore> = BTreeMap::new();
    for task in tasks {
        params
            .backend
            .index_page(&task.doc_id, &task.context, "context.txt", &Value::Null)
            .await?;
        for (q, a) in task.questions.iter().zip(task.answers.iter()) {
            let score = run_one(q, a, params).await?;
            let entry = by_split.entry(task.split.clone()).or_default();
            entry.total += 1;
            entry.hits += score;
        }
    }
    Ok(finalize(&by_split))
}

async fn run_one(question: &str, answer: &str, params: &RunParams) -> Result<f64, BenchError> {
    let hits = params
        .backend
        .search(question, MAX_K, &Filters::default(), StreamSet::all())
        .await?;
    let blob = hits
        .iter()
        .map(|h| h.snippet.clone())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    Ok(if blob.contains(&answer.to_lowercase()) {
        1.0
    } else {
        0.0
    })
}

#[derive(Default)]
struct SplitScore {
    total: usize,
    hits: f64,
}

fn finalize(by_split: &BTreeMap<String, SplitScore>) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let mut total_hits = 0.0;
    let mut total_n = 0usize;
    for (split, score) in by_split {
        if score.total == 0 {
            continue;
        }
        out.insert(format!("accuracy:{split}"), score.hits / safe_div(score.total));
        total_hits += score.hits;
        total_n += score.total;
    }
    if total_n > 0 {
        out.insert("accuracy".to_owned(), total_hits / safe_div(total_n));
    }
    out
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counts bounded by task count; well under 2^52"
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
            ("streams".to_owned(), Value::String("all".to_owned())),
        ]),
        metrics,
        cost_usd: 0.0,
        tokens_used: 0,
        wall_seconds: started.elapsed().as_secs(),
        timestamp: Utc::now(),
    }
}
