//! `BeamAdapter` — recall at 128K/500K/1M/10M; nugget-based scoring.

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

const K_VALUES: [usize; 3] = [1, 5, 10];
const MAX_K: usize = 10;

/// `BEAM` — Beyond a Million Tokens, ICLR 2026.
pub(crate) struct BeamAdapter;

#[async_trait]
impl BenchmarkAdapter for BeamAdapter {
    fn name(&self) -> &'static str {
        "beam"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let convos = load_conversations(params)?;
        let started = Instant::now();
        let metrics = score_conversations(&convos, params).await?;
        Ok(build_result(self.name(), params, metrics, started))
    }
}

struct Conversation {
    id: String,
    turns: Vec<String>,
    questions: Vec<Question>,
}

struct Question {
    text: String,
    answer: String,
    #[expect(dead_code, reason = "carried for future per-category breakouts")]
    kind: String,
}

fn load_conversations(params: &RunParams) -> Result<Vec<Conversation>, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full BEAM corpus loader not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "beam")?;
    array_field(&value, "conversations")?
        .iter()
        .map(parse_conversation)
        .collect()
}

fn parse_conversation(value: &Value) -> Result<Conversation, BenchError> {
    let id = string_field(value, "conv_id")?;
    let turns = parse_turns(value)?;
    let questions = parse_questions(value)?;
    Ok(Conversation { id, turns, questions })
}

fn parse_turns(value: &Value) -> Result<Vec<String>, BenchError> {
    Ok(array_field(value, "turns")?
        .iter()
        .filter_map(|t| t.get("content").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

fn parse_questions(value: &Value) -> Result<Vec<Question>, BenchError> {
    array_field(value, "questions")?
        .iter()
        .map(parse_question)
        .collect()
}

fn parse_question(value: &Value) -> Result<Question, BenchError> {
    Ok(Question {
        text: string_field(value, "question")?,
        answer: string_field(value, "answer")?,
        kind: string_field(value, "type")?,
    })
}

async fn score_conversations(
    convos: &[Conversation],
    params: &RunParams,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut hits: BTreeMap<usize, f64> = BTreeMap::new();
    let mut total = 0usize;
    for convo in convos {
        index_turns(convo, params).await?;
        for q in &convo.questions {
            total += 1;
            let snippets = run_query(q, params).await?;
            for k in K_VALUES {
                *hits.entry(k).or_default() += score_nugget(&q.answer, &snippets, k);
            }
        }
    }
    Ok(finalize(hits, total))
}

async fn index_turns(convo: &Conversation, params: &RunParams) -> Result<(), BenchError> {
    for (i, content) in convo.turns.iter().enumerate() {
        let id = format!("{}:t{i}", convo.id);
        let path = format!("conversations/{}.jsonl", convo.id);
        params
            .backend
            .index_page(&id, content, &path, &Value::Null)
            .await?;
    }
    Ok(())
}

async fn run_query(q: &Question, params: &RunParams) -> Result<Vec<String>, BenchError> {
    let hits = params
        .backend
        .search(&q.text, MAX_K, &Filters::default(), StreamSet::all())
        .await?;
    Ok(hits.into_iter().map(|h| h.snippet).collect())
}

/// Nugget score: lowercase answer tokens vs concatenated snippet text.
///
/// Each whitespace-separated token in the reference answer counts as a
/// "nugget"; the score is the fraction of nuggets that appear (case-
/// insensitive substring) anywhere in the top-K snippets. Rough but
/// rule-based — matches BEAM's "0 / 0.5 / 1 per atomic semantic unit"
/// without an LLM judge.
fn score_nugget(reference: &str, snippets: &[String], k: usize) -> f64 {
    let top = snippets
        .iter()
        .take(k)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let nuggets: Vec<String> = reference
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| !t.is_empty() && t.len() > 2)
        .collect();
    if nuggets.is_empty() {
        return 0.0;
    }
    let hits: usize = nuggets.iter().filter(|n| top.contains(n.as_str())).count();
    safe_div(hits, nuggets.len())
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counts bounded by question/answer size; well under 2^52"
)]
fn safe_div(num: usize, denom: usize) -> f64 {
    num as f64 / denom as f64
}

fn finalize(hits: BTreeMap<usize, f64>, n: usize) -> BTreeMap<String, f64> {
    if n == 0 {
        return BTreeMap::new();
    }
    let denom = safe_div(n, 1);
    hits.into_iter()
        .map(|(k, sum)| (format!("nugget@{k}"), sum / denom))
        .collect()
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
