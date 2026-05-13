//! Smoke-fixture loader for the MTEB adapter.
//!
//! Reads `benchmarks/mteb-retrieval/fixtures/smoke.json` and materialises it
//! into the same [`Corpus`] shape the BEIR loader produces, so `score_queries`
//! has a single code path for both modes.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::{adapter::load_smoke_fixture, error::BenchError};

use super::corpus::{Corpus, Document, Qrels, Query};

const BENCH_NAME: &str = "mteb-retrieval";

pub(super) fn load_smoke_corpus(corpus_dir: &Path) -> Result<Corpus, BenchError> {
    let value = load_smoke_fixture(corpus_dir, BENCH_NAME)?;
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

fn parse_qrels(value: &Value) -> Result<Qrels, BenchError> {
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
