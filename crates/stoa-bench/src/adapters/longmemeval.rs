//! `LongMemEval` adapter — real implementation against `RecallBackend`.

use std::collections::{BTreeMap, HashSet};
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

/// `LongMemEval` — five-category multi-session recall + reasoning.
///
/// Source: Wu et al. 2024, <https://arxiv.org/abs/2410.10813>.
/// Corpus: `xiaowu0162/longmemeval-cleaned` on `HuggingFace`.
/// Metrics: `recall@1`, `recall@5`, `recall@10` per question category.
pub(crate) struct LongmemEvalAdapter;

const K_VALUES: [usize; 3] = [1, 5, 10];
const MAX_K: usize = 10;

#[async_trait]
impl BenchmarkAdapter for LongmemEvalAdapter {
    fn name(&self) -> &'static str {
        "longmemeval"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let questions = load_questions(params)?;
        let started = Instant::now();
        let metrics = score_questions(&questions, params).await?;
        Ok(build_result(self.name(), params, metrics, started))
    }
}

/// One `LongMemEval` question with its haystack sessions.
struct Question {
    #[expect(
        dead_code,
        reason = "carried for debugging + future scorer integration"
    )]
    id: String,
    text: String,
    haystack_session_ids: Vec<String>,
    sessions: Vec<Session>,
    answer_session_ids: HashSet<String>,
}

/// One conversation session in the haystack.
struct Session {
    id: String,
    turns: Vec<String>,
}

fn load_questions(params: &RunParams) -> Result<Vec<Question>, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full LongMemEval corpus load not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "longmemeval")?;
    let array = value
        .as_array()
        .ok_or_else(|| BenchError::CorpusParse("expected JSON array".to_owned()))?;
    array.iter().map(parse_question).collect()
}

fn parse_question(value: &Value) -> Result<Question, BenchError> {
    let id = string_field(value, "question_id")?;
    let text = string_field(value, "question")?;
    let haystack_session_ids = string_array(value, "haystack_session_ids")?;
    let sessions = parse_sessions(value, &haystack_session_ids)?;
    let answer_session_ids = string_array(value, "answer_session_ids")?
        .into_iter()
        .collect();
    Ok(Question {
        id,
        text,
        haystack_session_ids,
        sessions,
        answer_session_ids,
    })
}

fn parse_sessions(value: &Value, ids: &[String]) -> Result<Vec<Session>, BenchError> {
    let haystacks = value
        .get("haystack_sessions")
        .and_then(Value::as_array)
        .ok_or_else(|| BenchError::CorpusParse("missing haystack_sessions".to_owned()))?;
    haystacks
        .iter()
        .zip(ids.iter())
        .map(|(session_value, sid)| parse_session(sid.clone(), session_value))
        .collect()
}

fn parse_session(id: String, value: &Value) -> Result<Session, BenchError> {
    let turns_value = value
        .as_array()
        .ok_or_else(|| BenchError::CorpusParse(format!("session {id} not an array")))?;
    let turns = turns_value
        .iter()
        .filter_map(|turn| {
            turn.get("content")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    Ok(Session { id, turns })
}

fn string_field(value: &Value, key: &str) -> Result<String, BenchError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing string field `{key}`")))
}

fn string_array(value: &Value, key: &str) -> Result<Vec<String>, BenchError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| BenchError::CorpusParse(format!("missing array field `{key}`")))
}

async fn score_questions(
    questions: &[Question],
    params: &RunParams,
) -> Result<BTreeMap<String, f64>, BenchError> {
    index_corpus(questions, params).await?;
    let mut totals: BTreeMap<usize, f64> = BTreeMap::new();
    for q in questions {
        let session_ids = run_query(q, params).await?;
        for k in K_VALUES {
            let hit = answer_in_top_k(&session_ids, &q.answer_session_ids, k);
            *totals.entry(k).or_default() += f64::from(u8::from(hit));
        }
    }
    Ok(finalize_metrics(totals, questions.len()))
}

async fn index_corpus(questions: &[Question], params: &RunParams) -> Result<(), BenchError> {
    let mut seen = HashSet::new();
    for q in questions {
        for session in &q.sessions {
            if !seen.insert(session.id.clone()) {
                continue;
            }
            let content = session.turns.join("\n");
            let path = format!("sessions/{}.jsonl", session.id);
            params
                .backend
                .index_page(&session.id, &content, &path, &Value::Null)
                .await?;
        }
    }
    Ok(())
}

async fn run_query(q: &Question, params: &RunParams) -> Result<Vec<String>, BenchError> {
    let hits = params
        .backend
        .search(&q.text, MAX_K, &Filters::default(), StreamSet::all())
        .await?;
    Ok(hits
        .into_iter()
        .filter(|h| {
            q.haystack_session_ids
                .iter()
                .any(|sid| sid == h.doc_id.as_str())
        })
        .map(|h| h.doc_id.as_str().to_owned())
        .collect())
}

fn answer_in_top_k(retrieved: &[String], answers: &HashSet<String>, k: usize) -> bool {
    retrieved.iter().take(k).any(|sid| answers.contains(sid))
}

fn finalize_metrics(totals: BTreeMap<usize, f64>, n: usize) -> BTreeMap<String, f64> {
    if n == 0 {
        return BTreeMap::new();
    }
    let denom = precision_lossy_usize_to_f64(n);
    totals
        .into_iter()
        .map(|(k, hits)| (format!("recall@{k}"), hits / denom))
        .collect()
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

#[expect(
    clippy::cast_precision_loss,
    reason = "n is bounded by available question count — well below 2^52"
)]
fn precision_lossy_usize_to_f64(n: usize) -> f64 {
    n as f64
}
