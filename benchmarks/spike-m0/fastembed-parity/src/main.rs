// Embed a fixed corpus with fastembed (Rust, ONNX via ort) using bge-small-en-v1.5.
// Writes JSON: [{"text": ..., "embedding": [..]} ...]
// Also prints throughput (texts/sec, embed time excluding model load).

use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::time::Instant;

const CORPUS: &[&str] = &[
    "Stoa is an open-core memory system for AI agents.",
    "The painted porch was a public space for philosophical discussion in ancient Athens.",
    "Hybrid recall combines vector search with BM25 lexical scoring.",
    "Reciprocal rank fusion merges multiple ranked lists into one.",
    "ONNX Runtime executes models exported from PyTorch or TensorFlow.",
    "WAL mode in SQLite allows concurrent readers and a single writer.",
    "MINJA stands for memory injection attack.",
    "Treisman's pre-attentive processing model classifies visual features by latency.",
    "Cleveland and McGill ranked perceptual tasks by accuracy in 1984.",
    "The bge-small-en-v1.5 model produces 384-dimensional embeddings.",
    "Cross-compilation from Linux to Apple Silicon requires the macOS SDK.",
    "Reciprocal rank fusion uses k=60 by default in published literature.",
];

fn main() -> Result<()> {
    let t0 = Instant::now();
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
    )?;
    let load_ms = t0.elapsed().as_millis();
    eprintln!("model_load_ms={load_ms}");

    let texts: Vec<String> = CORPUS.iter().map(|s| s.to_string()).collect();

    // Warm up
    let _ = model.embed(texts.clone(), None)?;

    // Measured run
    let t1 = Instant::now();
    let embeddings = model.embed(texts.clone(), None)?;
    let embed_us = t1.elapsed().as_micros();
    let n = embeddings.len();
    let throughput = (n as f64) / (embed_us as f64 / 1_000_000.0);
    eprintln!("n={n} embed_us={embed_us} throughput_texts_per_sec={throughput:.1}");

    // Per-text timing
    let t2 = Instant::now();
    for t in &texts {
        let _ = model.embed(vec![t.clone()], None)?;
    }
    let single_us = t2.elapsed().as_micros();
    let per_text_us = single_us / n as u128;
    eprintln!("single_text_avg_us={per_text_us}");

    // Output JSON to stdout for parity comparison
    let out: Vec<serde_json::Value> = texts
        .iter()
        .zip(embeddings.iter())
        .map(|(t, e)| {
            serde_json::json!({
                "text": t,
                "embedding": e,
            })
        })
        .collect();
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
