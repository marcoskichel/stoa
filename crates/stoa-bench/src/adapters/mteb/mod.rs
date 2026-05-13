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

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use stoa_recall::{Filters, RecallBackend, StreamSet};
use stoa_recall_local_chroma_sqlite::Bm25Backend;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    cli::BackendKind,
    error::BenchError,
    result::BenchmarkResult,
};

use corpus::{Corpus, Document, Query};
use scoring::{K_VALUES, NDCG_K, TOP_K, as_f64, ndcg_at_k, recall_at_k};

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
    run_one_subset("smoke", &corpus, &*params.backend).await
}

/// Run every BEIR subset and return the per-subset + aggregated metrics.
///
/// Each subset gets a **fresh BM25 index** in a per-PID tempdir when the
/// backend is `local-chroma-sqlite` — the canonical BEIR evaluation
/// methodology runs every subset against its own corpus, so IDF stays
/// computed over a single domain's lexicon. For the `no-memory` control
/// arm the supplied `NoopBackend` is reused across subsets (it indexes
/// nothing, so isolation is moot).
///
/// Doc-ids and qrel keys are namespaced with the subset name (e.g.
/// `scifact:d0`) as defense in depth: even when a backend is reused
/// across subsets (smoke / control arm), id collisions between subsets
/// cannot make one subset's row overwrite another's.
async fn run_full(params: &RunParams) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut per_subset: Vec<(String, BTreeMap<String, f64>)> = Vec::new();
    let isolate = isolated_per_subset(&params.backend_name);
    let shared_dir = if isolate {
        Some(mteb_temp_root()?)
    } else {
        None
    };
    for subset in beir::SUBSETS {
        let corpus = beir::load_subset(&params.corpus_dir, subset)?;
        let metrics = match shared_dir.as_deref() {
            Some(root) => run_subset_isolated(subset, &corpus, root).await?,
            None => run_one_subset(subset, &corpus, &*params.backend).await?,
        };
        per_subset.push((subset.to_owned(), metrics));
    }
    Ok(aggregate(per_subset))
}

fn isolated_per_subset(backend_name: &str) -> bool {
    backend_name == BackendKind::LocalChromaSqlite.as_str()
}

/// Index `corpus` into a brand-new `Bm25Backend` rooted under `root`,
/// then score `corpus.queries` against it.
///
/// The on-disk `.db` file is removed when the function returns so the
/// next subset starts from a clean FTS5 index — no doc-id collisions,
/// no cross-subset IDF contamination.
async fn run_subset_isolated(
    subset: &str,
    corpus: &Corpus,
    root: &std::path::Path,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let db_path = root.join(format!("{subset}.db"));
    drop(std::fs::remove_file(&db_path));
    let backend = Bm25Backend::open(&db_path)
        .map_err(|e| BenchError::Backend(format!("open per-subset bm25: {e}")))?;
    let arc: Arc<dyn RecallBackend> = Arc::new(backend);
    let metrics = run_one_subset(subset, corpus, &*arc).await?;
    drop(arc);
    drop(std::fs::remove_file(&db_path));
    Ok(metrics)
}

/// Per-PID tempdir for the per-subset BM25 indices.
///
/// Uses the process id so concurrent runs (CI matrix) cannot stomp on
/// each other; cleared on entry so a previous crash's residue cannot
/// poison the fresh run.
fn mteb_temp_root() -> Result<PathBuf, BenchError> {
    let dir = std::env::temp_dir().join(format!("stoa-bench-mteb-{}", std::process::id()));
    if dir.exists() {
        drop(std::fs::remove_dir_all(&dir));
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

async fn run_one_subset(
    subset: &str,
    corpus: &Corpus,
    backend: &dyn RecallBackend,
) -> Result<BTreeMap<String, f64>, BenchError> {
    index_documents(subset, &corpus.documents, backend).await?;
    score_queries(subset, corpus, backend).await
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
            out.insert(metric, sum / as_f64(n));
        }
    }
    out
}

async fn index_documents(
    subset: &str,
    docs: &[Document],
    backend: &dyn RecallBackend,
) -> Result<(), BenchError> {
    for doc in docs {
        let namespaced = namespaced_id(subset, &doc.id);
        let path = format!("corpus/{subset}/{}.txt", doc.id);
        backend
            .index_page(&namespaced, &doc.text, &path, &Value::Null)
            .await?;
    }
    Ok(())
}

async fn score_queries(
    subset: &str,
    corpus: &Corpus,
    backend: &dyn RecallBackend,
) -> Result<BTreeMap<String, f64>, BenchError> {
    let mut ndcg_sum = 0.0;
    let mut recall_sums: BTreeMap<usize, f64> = BTreeMap::new();
    let mut counted: usize = 0;
    for query in &corpus.queries {
        let Some(qrels) = corpus.qrels.get(&query.id) else {
            continue;
        };
        let ns_qrels = namespaced_qrels(subset, qrels);
        let hits = run_search(query, backend).await?;
        ndcg_sum += ndcg_at_k(&hits, &ns_qrels, NDCG_K);
        for k in K_VALUES {
            *recall_sums.entry(k).or_default() += recall_at_k(&hits, &ns_qrels, k);
        }
        counted += 1;
    }
    Ok(finalize_metrics(ndcg_sum, recall_sums, counted))
}

async fn run_search(query: &Query, backend: &dyn RecallBackend) -> Result<Vec<String>, BenchError> {
    let hits = backend
        .search(&query.text, TOP_K, &Filters::default(), search_streams())
        .await?;
    Ok(hits
        .into_iter()
        .map(|h| h.doc_id.as_str().to_owned())
        .collect())
}

/// Prepend the subset name to a BEIR `_id` so cross-subset doc-id
/// collisions cannot make one subset's row in the shared FTS5 index
/// overwrite another's.
fn namespaced_id(subset: &str, doc_id: &str) -> String {
    format!("{subset}:{doc_id}")
}

/// Re-key one query's qrels into the same namespace [`index_documents`]
/// writes under, so qrels lookups against retrieved hits still match.
fn namespaced_qrels(subset: &str, qrels: &HashMap<String, u32>) -> HashMap<String, u32> {
    qrels
        .iter()
        .map(|(did, &score)| (namespaced_id(subset, did), score))
        .collect()
}

/// Streams the MTEB adapter queries against.
///
/// Pinned to BM25-only. The v0.1 default backend exposes BM25 as the
/// always-on leg; vector / graph streams would route via the Python
/// sidecar and at BEIR scale (~57k fiqa queries) any per-query timeout
/// becomes hours of dead wait when the sidecar is offline. Restricting
/// to BM25 keeps the bench self-contained and the reported `streams`
/// hyperparam (`"bm25"`) honest about what actually ran.
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
    let denom = as_f64(n);
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
        return "smoke".to_owned();
    }
    let subsets_label = format!("beir-{}", beir::SUBSETS.join("+"));
    let version_path = params.corpus_dir.join("mteb-retrieval").join(".version");
    match std::fs::read_to_string(&version_path) {
        Ok(s) => format!("{subsets_label}@{}", s.trim()),
        Err(_) => subsets_label,
    }
}

fn hyperparams() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("k".to_owned(), Value::Number(serde_json::Number::from(NDCG_K))),
        ("streams".to_owned(), Value::String("bm25".to_owned())),
    ])
}
