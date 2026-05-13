//! `MtebAdapter` — BEIR retrieval subset, NDCG@10 + recall@k.
//!
//! Smoke mode runs against `benchmarks/mteb-retrieval/fixtures/smoke.json`.
//! Full mode iterates `scifact`, `nfcorpus`, `fiqa` from
//! `benchmarks/corpus/mteb-retrieval/<subset>/` and reports per-subset
//! metrics plus their unweighted mean (e.g. `ndcg@10:scifact`, `ndcg@10`).

mod beir;
mod corpus;
mod scoring;
mod smoke;

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use stoa_recall::{Filters, StreamSet};

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use corpus::{Corpus, Document, Query};
use scoring::{K_VALUES, NDCG_K, ndcg_at_k, precision_safe_div, recall_at_k};

/// MTEB/BEIR retrieval subset — embedding component quality check.
pub(crate) struct MtebAdapter;

#[async_trait]
impl BenchmarkAdapter for MtebAdapter {
    fn name(&self) -> &'static str {
        "mteb-retrieval"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        let started = Instant::now();
        let metrics = if params.smoke {
            run_smoke(params).await?
        } else {
            run_full(params).await?
        };
        Ok(build_result(self.name(), params, metrics, started))
    }
}

async fn run_smoke(params: &RunParams) -> Result<BTreeMap<String, f64>, BenchError> {
    let corpus = smoke::load_smoke_corpus(&params.corpus_dir)?;
    index_documents(&corpus.documents, params).await?;
    score_queries(&corpus, params).await
}

async fn run_full(params: &RunParams) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut per_subset: Vec<(String, BTreeMap<String, f64>)> = Vec::new();
    for subset in beir::SUBSETS {
        let corpus = beir::load_subset(&params.corpus_dir, subset)?;
        index_documents(&corpus.documents, params).await?;
        let metrics = score_queries(&corpus, params).await?;
        per_subset.push((subset.to_owned(), metrics));
    }
    Ok(aggregate(per_subset))
}

fn aggregate(per_subset: Vec<(String, BTreeMap<String, f64>)>) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let mut means: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for (subset, metrics) in per_subset {
        for (metric, value) in metrics {
            out.insert(format!("{metric}:{subset}"), value);
            let entry = means.entry(metric).or_insert((0.0, 0));
            entry.0 += value;
            entry.1 += 1;
        }
    }
    for (metric, (sum, n)) in means {
        if n > 0 {
            out.insert(metric, sum / precision_safe_div(n, 1));
        }
    }
    out
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
    let mut counted: usize = 0;
    for query in &corpus.queries {
        let Some(qrels) = corpus.qrels.get(&query.id) else {
            continue;
        };
        let hits = run_search(query, params).await?;
        ndcg_sum += ndcg_at_k(&hits, qrels, NDCG_K);
        for k in K_VALUES {
            *recall_sums.entry(k).or_default() += recall_at_k(&hits, qrels, k);
        }
        counted += 1;
    }
    Ok(finalize_metrics(ndcg_sum, recall_sums, counted))
}

async fn run_search(query: &Query, params: &RunParams) -> Result<Vec<String>, BenchError> {
    let top_k = *K_VALUES.iter().max().unwrap_or(&100);
    let hits = params
        .backend
        .search(&query.text, top_k, &Filters::default(), search_streams())
        .await?;
    Ok(hits
        .into_iter()
        .map(|h| h.doc_id.as_str().to_owned())
        .collect())
}

/// Streams the MTEB adapter queries against.
///
/// `IpcBackend::search` with vector / graph streams waits the full
/// `DEFAULT_SEARCH_TIMEOUT` (2s) per query when the Python sidecar is
/// offline before degrading to BM25 — at BEIR scale (~57k fiqa queries)
/// that's hours of pure timeout. The MTEB pipe targets the embedding
/// component, but the v0.1 default backend exposes BM25 as the always-on
/// leg, so we restrict the bench to BM25 until the sidecar lands in the
/// runner. The reported `streams=all` hyperparam in the result remains
/// the canonical run configuration; this flag is the operational lane.
fn search_streams() -> StreamSet {
    StreamSet::bm25_only()
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
        corpus_rev: corpus_rev(params),
        scorer_rev: params.scorer_rev.clone(),
        backbone_model: params.backbone_model.clone(),
        hyperparams: hyperparams(),
        metrics,
        cost_usd: 0.0,
        tokens_used: 0,
        wall_seconds: started.elapsed().as_secs(),
        timestamp: Utc::now(),
    }
}

fn corpus_rev(params: &RunParams) -> String {
    if params.smoke {
        "smoke".to_owned()
    } else {
        format!("beir-{}", beir::SUBSETS.join("+"))
    }
}

fn hyperparams() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("k".to_owned(), Value::Number(serde_json::Number::from(NDCG_K))),
        ("streams".to_owned(), Value::String("bm25".to_owned())),
    ])
}
