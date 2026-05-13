//! `MtebAdapter` — BEIR retrieval subset, NDCG@10.

use std::collections::{BTreeMap, HashMap};
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

const K_VALUES: [usize; 3] = [1, 10, 100];
const NDCG_K: usize = 10;

/// MTEB/BEIR retrieval subset — embedding component quality check.
pub(crate) struct MtebAdapter;

#[async_trait]
impl BenchmarkAdapter for MtebAdapter {
    fn name(&self) -> &'static str {
        "mteb-retrieval"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let corpus = load_corpus(params)?;
        let started = Instant::now();
        index_documents(&corpus.documents, params).await?;
        let metrics = score_queries(&corpus, params).await?;
        Ok(build_result(self.name(), params, metrics, started))
    }
}

struct Corpus {
    documents: Vec<Document>,
    queries: Vec<Query>,
    qrels: HashMap<String, HashMap<String, u32>>,
}

struct Document {
    id: String,
    text: String,
}

struct Query {
    id: String,
    text: String,
}

fn load_corpus(params: &RunParams) -> Result<Corpus, BenchError> {
    if !params.smoke {
        return Err(BenchError::CorpusParse(
            "full BEIR corpus loader not yet wired — pass --smoke".to_owned(),
        ));
    }
    let value = load_smoke_fixture(&params.corpus_dir, "mteb-retrieval")?;
    Ok(Corpus {
        documents: parse_documents(&value)?,
        queries: parse_queries(&value)?,
        qrels: parse_qrels(&value)?,
    })
}

fn parse_documents(value: &Value) -> Result<Vec<Document>, BenchError> {
    array_field(value, "corpus")?
        .iter()
        .map(|d| {
            Ok(Document {
                id: string_field(d, "_id")?,
                text: string_field(d, "text")?,
            })
        })
        .collect()
}

fn parse_queries(value: &Value) -> Result<Vec<Query>, BenchError> {
    array_field(value, "queries")?
        .iter()
        .map(|q| {
            Ok(Query {
                id: string_field(q, "_id")?,
                text: string_field(q, "text")?,
            })
        })
        .collect()
}

fn parse_qrels(value: &Value) -> Result<HashMap<String, HashMap<String, u32>>, BenchError> {
    let obj = value
        .get("qrels")
        .and_then(Value::as_object)
        .ok_or_else(|| BenchError::CorpusParse("missing qrels object".to_owned()))?;
    obj.iter().map(parse_one_qrel).collect()
}

fn parse_one_qrel(
    (qid, rels): (&String, &Value),
) -> Result<(String, HashMap<String, u32>), BenchError> {
    let map = rels
        .as_object()
        .ok_or_else(|| BenchError::CorpusParse(format!("qrels[{qid}] not object")))?;
    let inner = map
        .iter()
        .filter_map(|(did, r)| {
            r.as_u64()
                .map(|v| (did.clone(), u32::try_from(v).unwrap_or(0)))
        })
        .collect();
    Ok((qid.clone(), inner))
}

async fn index_documents(docs: &[Document], params: &RunParams) -> Result<(), BenchError> {
    for doc in docs {
        let path = format!("corpus/{}.txt", doc.id);
        params
            .backend
            .index_page(&doc.id, &doc.text, &path, &Value::Null)
            .await?;
    }
    Ok(())
}

async fn score_queries(
    corpus: &Corpus,
    params: &RunParams,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut ndcg_sum = 0.0;
    let mut recall_sums: BTreeMap<usize, f64> = BTreeMap::new();
    for query in &corpus.queries {
        let hits = run_search(query, params).await?;
        let empty = HashMap::new();
        let qrels = corpus.qrels.get(&query.id).unwrap_or(&empty);
        ndcg_sum += ndcg_at_k(&hits, qrels, NDCG_K);
        for k in K_VALUES {
            *recall_sums.entry(k).or_default() += recall_at_k(&hits, qrels, k);
        }
    }
    Ok(finalize_metrics(ndcg_sum, recall_sums, corpus.queries.len()))
}

async fn run_search(query: &Query, params: &RunParams) -> Result<Vec<String>, BenchError> {
    let hits = params
        .backend
        .search(
            &query.text,
            *K_VALUES.last().unwrap_or(&100),
            &Filters::default(),
            StreamSet::all(),
        )
        .await?;
    Ok(hits
        .into_iter()
        .map(|h| h.doc_id.as_str().to_owned())
        .collect())
}

fn recall_at_k(retrieved: &[String], qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let relevant: usize = qrels.values().filter(|&&v| v > 0).count();
    if relevant == 0 {
        return 0.0;
    }
    let hits: usize = retrieved
        .iter()
        .take(k)
        .filter(|did| qrels.get(*did).copied().unwrap_or(0) > 0)
        .count();
    precision_safe_div(hits, relevant)
}

fn ndcg_at_k(retrieved: &[String], qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let mut dcg = 0.0;
    for (rank, doc_id) in retrieved.iter().take(k).enumerate() {
        let rel = f64::from(qrels.get(doc_id).copied().unwrap_or(0));
        if rel > 0.0 {
            dcg += rel / log2_rank(rank);
        }
    }
    let idcg = ideal_dcg(qrels, k);
    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

fn ideal_dcg(qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let mut rels: Vec<u32> = qrels.values().copied().collect();
    rels.sort_unstable_by(|a, b| b.cmp(a));
    rels.iter()
        .take(k)
        .enumerate()
        .map(|(rank, r)| f64::from(*r) / log2_rank(rank))
        .sum()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "rank index is small; precision loss is irrelevant for DCG denominator"
)]
fn log2_rank(rank: usize) -> f64 {
    ((rank + 2) as f64).log2()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hit + total counts bounded by corpus size; well under 2^52"
)]
fn precision_safe_div(num: usize, denom: usize) -> f64 {
    num as f64 / denom as f64
}

fn finalize_metrics(
    ndcg_sum: f64,
    recall_sums: BTreeMap<usize, f64>,
    n: usize,
) -> BTreeMap<String, f64> {
    if n == 0 {
        return BTreeMap::new();
    }
    let denom = precision_safe_div(n, 1);
    let mut out = BTreeMap::new();
    out.insert("ndcg@10".to_owned(), ndcg_sum / denom);
    for (k, sum) in recall_sums {
        out.insert(format!("recall@{k}"), sum / denom);
    }
    out
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
            ("k".to_owned(), Value::Number(serde_json::Number::from(NDCG_K))),
            ("streams".to_owned(), Value::String("all".to_owned())),
        ]),
        metrics,
        cost_usd: 0.0,
        tokens_used: 0,
        wall_seconds: started.elapsed().as_secs(),
        timestamp: Utc::now(),
    }
}
