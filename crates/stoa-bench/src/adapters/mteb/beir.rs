//! BEIR canonical-layout loader (jsonl + tsv qrels) for one subset.
//!
//! Layout expected under `<corpus_root>/mteb-retrieval/<subset>/`:
//! ```text
//! corpus.jsonl[.gz]       {_id, title, text}
//! queries.jsonl[.gz]      {_id, title, text}
//! qrels/test.tsv          query-id<TAB>corpus-id<TAB>score (header line skipped)
//! ```
//!
//! The downloader in `benchmarks/corpus/mteb-retrieval.sh` populates this
//! directory from the canonical BEIR mirror (which still ships jsonl/tsv;
//! the Hugging Face `BeIR/<name>` mirrors have migrated to parquet).

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use serde_json::Value;

use crate::error::BenchError;

use super::corpus::{Corpus, Document, Qrels, Query};

/// Subsets aggregated for the `--bench mteb` full run.
pub(super) const SUBSETS: [&str; 3] = ["scifact", "nfcorpus", "fiqa"];

/// Load corpus + queries + qrels for `subset` from `corpus_root`.
pub(super) fn load_subset(corpus_root: &Path, subset: &str) -> Result<Corpus, BenchError> {
    let subset_dir = corpus_root.join("mteb-retrieval").join(subset);
    if !subset_dir.is_dir() {
        let path = subset_dir.to_string_lossy().into_owned();
        return Err(BenchError::CorpusMissing { path });
    }
    let documents = read_jsonl(&subset_dir, "corpus", parse_document)?;
    let queries = read_jsonl(&subset_dir, "queries", parse_query)?;
    let qrels = read_qrels(&subset_dir)?;
    Ok(Corpus { documents, queries, qrels })
}

fn parse_document(value: &Value) -> Result<Document, BenchError> {
    Ok(Document {
        id: string_field(value, "_id")?,
        text: combine_text(value),
    })
}

fn parse_query(value: &Value) -> Result<Query, BenchError> {
    Ok(Query {
        id: string_field(value, "_id")?,
        text: combine_text(value),
    })
}

fn combine_text(value: &Value) -> String {
    let title = optional_string(value, "title");
    let text = optional_string(value, "text");
    match (title.is_empty(), text.is_empty()) {
        (true, _) => text,
        (false, true) => title,
        (false, false) => format!("{title}. {text}"),
    }
}

fn read_jsonl<T, F>(dir: &Path, stem: &str, parse: F) -> Result<Vec<T>, BenchError>
where
    F: Fn(&Value) -> Result<T, BenchError>,
{
    let path = resolve_jsonl_path(dir, stem)?;
    let reader = open_jsonl(&path)?;
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)?;
        out.push(parse(&value)?);
    }
    Ok(out)
}

fn resolve_jsonl_path(dir: &Path, stem: &str) -> Result<PathBuf, BenchError> {
    let plain = dir.join(format!("{stem}.jsonl"));
    if plain.exists() {
        return Ok(plain);
    }
    let gz = dir.join(format!("{stem}.jsonl.gz"));
    if gz.exists() {
        return Ok(gz);
    }
    let path = plain.to_string_lossy().into_owned();
    Err(BenchError::CorpusMissing { path })
}

fn open_jsonl(path: &Path) -> Result<Box<dyn BufRead>, BenchError> {
    let file = File::open(path)?;
    let raw: Box<dyn Read> = if is_gzip(path) {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    Ok(Box::new(BufReader::new(raw)))
}

fn is_gzip(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "gz")
}

fn read_qrels(dir: &Path) -> Result<Qrels, BenchError> {
    let path = dir.join("qrels").join("test.tsv");
    if !path.exists() {
        let p = path.to_string_lossy().into_owned();
        return Err(BenchError::CorpusMissing { path: p });
    }
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut out: Qrels = HashMap::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        ingest_qrel_row(&mut out, idx, &line)?;
    }
    Ok(out)
}

fn ingest_qrel_row(out: &mut Qrels, idx: usize, line: &str) -> Result<(), BenchError> {
    if line.trim().is_empty() {
        return Ok(());
    }
    if idx == 0 && looks_like_header(line) {
        return Ok(());
    }
    let (qid, did, score) = split_qrel_row(line)?;
    out.entry(qid).or_default().insert(did, score);
    Ok(())
}

fn looks_like_header(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("query-id") || lower.contains("query_id") || lower.contains("qid")
}

fn split_qrel_row(line: &str) -> Result<(String, String, u32), BenchError> {
    let mut parts = line.split('\t');
    let qid = parts
        .next()
        .ok_or_else(|| BenchError::CorpusParse(format!("qrels row missing qid: `{line}`")))?;
    let did = parts
        .next()
        .ok_or_else(|| BenchError::CorpusParse(format!("qrels row missing did: `{line}`")))?;
    let raw = parts
        .next()
        .ok_or_else(|| BenchError::CorpusParse(format!("qrels row missing score: `{line}`")))?;
    let score: u32 = raw
        .trim()
        .parse()
        .map_err(|e| BenchError::CorpusParse(format!("qrels row `{line}` score: {e}")))?;
    Ok((qid.to_owned(), did.to_owned(), score))
}

fn string_field(value: &Value, key: &str) -> Result<String, BenchError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BenchError::CorpusParse(format!("missing string `{key}`")))
}

fn optional_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
