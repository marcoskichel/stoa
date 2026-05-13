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

/// Per-line cap on the decoded stream — rejects hostile records without
/// blowing memory. Real BEIR scenarios are well under 64 KiB each, so
/// 1 MiB is a generous safety margin.
const MAX_LINE_BYTES: usize = 1 << 20;
/// Hard ceiling on total decoded bytes per file. A gzip bomb that stays
/// just under [`MAX_LINE_BYTES`] per line would still inflate without
/// bound; this stops the read regardless of per-line shape.
const MAX_DECODED_BYTES: u64 = 5 * 1024 * 1024 * 1024;
/// Maximum tolerated malformed / over-long lines before the load aborts.
const MAX_SKIPS: usize = 16;

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

/// Stream a BEIR jsonl file with hostile-input guards.
///
/// Each decoded line is bounded by [`MAX_LINE_BYTES`] (over-long lines
/// are skipped with a `tracing::warn!` and a counter; exceeding
/// [`MAX_SKIPS`] aborts the load). The whole decoded stream is bounded
/// by [`MAX_DECODED_BYTES`] so a slow-drip gzip bomb cannot bypass the
/// per-line cap by keeping every line just under it.
fn read_jsonl<T, F>(dir: &Path, stem: &str, parse: F) -> Result<Vec<T>, BenchError>
where
    F: Fn(&Value) -> Result<T, BenchError>,
{
    let path = resolve_jsonl_path(dir, stem)?;
    let mut reader = open_jsonl(&path)?;
    let mut out = Vec::new();
    let mut skipped: usize = 0;
    let mut buf: Vec<u8> = Vec::with_capacity(4 * 1024);
    loop {
        buf.clear();
        if !read_bounded_line(reader.as_mut(), &mut buf, &mut skipped)? {
            break;
        }
        if let Some(value) = parse_line(&buf, &mut skipped)? {
            out.push(parse(&value)?);
        }
        if skipped > MAX_SKIPS {
            return Err(BenchError::CorpusParse(format!(
                "skip count exceeded {MAX_SKIPS} in {}; corpus likely corrupt",
                path.display(),
            )));
        }
    }
    Ok(out)
}

/// Read one newline-terminated record into `buf`, enforcing
/// [`MAX_LINE_BYTES`]. Returns `Ok(false)` at EOF.
///
/// Over-long lines are drained to the next `\n` then skipped — keeps
/// the read aligned on record boundaries instead of corrupting the
/// next read.
fn read_bounded_line(
    reader: &mut dyn BufRead,
    buf: &mut Vec<u8>,
    skipped: &mut usize,
) -> Result<bool, BenchError> {
    let cap = u64::try_from(MAX_LINE_BYTES).unwrap_or(u64::MAX);
    let read = (&mut *reader).take(cap).read_until(b'\n', buf)?;
    if read == 0 {
        return Ok(false);
    }
    if read == MAX_LINE_BYTES && !ends_with_newline(buf) {
        *skipped += 1;
        tracing::warn!(target: "stoa_bench::mteb", bytes = read, "skipping over-long jsonl line");
        drain_to_newline(reader)?;
        buf.clear();
    }
    Ok(true)
}

fn ends_with_newline(buf: &[u8]) -> bool {
    buf.last().copied() == Some(b'\n')
}

fn drain_to_newline(reader: &mut dyn BufRead) -> Result<(), BenchError> {
    let mut sink: Vec<u8> = Vec::new();
    let _ = reader.read_until(b'\n', &mut sink)?;
    Ok(())
}

fn parse_line(buf: &[u8], skipped: &mut usize) -> Result<Option<Value>, BenchError> {
    if buf.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    let Ok(text) = std::str::from_utf8(buf) else {
        *skipped += 1;
        tracing::warn!(target: "stoa_bench::mteb", "skipping non-utf8 jsonl line");
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(text)?))
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
        Box::new(GzDecoder::new(file).take(MAX_DECODED_BYTES))
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
    let mut first_data_row_seen = false;
    for line in reader.lines() {
        let line = line?;
        first_data_row_seen = ingest_qrel_row(&mut out, &line, first_data_row_seen)?;
    }
    Ok(out)
}

/// Ingest one qrels TSV row.
///
/// Skips blank lines and skips the FIRST non-empty row when it looks
/// like a header — BEIR's canonical mirror ships a `query-id\tcorpus-id\tscore`
/// header, but UTF-8 BOM bytes or a leading blank line can shift the
/// header to a row other than index 0. `first_data_row_seen` carries the
/// "already past any header" state across the iteration.
fn ingest_qrel_row(
    out: &mut Qrels,
    line: &str,
    first_data_row_seen: bool,
) -> Result<bool, BenchError> {
    let trimmed = strip_bom(line);
    if trimmed.trim().is_empty() {
        return Ok(first_data_row_seen);
    }
    if !first_data_row_seen && looks_like_header(trimmed) {
        return Ok(true);
    }
    let (qid, did, score) = split_qrel_row(trimmed)?;
    out.entry(qid).or_default().insert(did, score);
    Ok(true)
}

/// Strip a leading UTF-8 BOM (`EF BB BF`) so the first header column
/// can be matched verbatim.
fn strip_bom(line: &str) -> &str {
    line.strip_prefix('\u{feff}').unwrap_or(line)
}

/// Detect a BEIR-canonical TSV header row.
///
/// Matches the first tab-delimited token against one of the well-known
/// query-id column names (case-insensitive). Substring matching on
/// "qid" anywhere in the row would treat a data row like
/// `mqid42<TAB>doc7<TAB>1` as a header.
fn looks_like_header(line: &str) -> bool {
    let first = line.split('\t').next().unwrap_or("").trim().to_lowercase();
    matches!(first.as_str(), "query-id" | "query_id" | "qid" | "queryid")
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
