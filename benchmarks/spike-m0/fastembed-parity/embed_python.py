"""Python reference embeddings using fastembed (PyPI).

Embeds the same fixed corpus as embed-rust, writes JSON to stdout,
and prints model load + embed throughput to stderr.
"""

import json
import sys
import time

from fastembed import TextEmbedding

CORPUS = [
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
]


def main() -> None:
    t0 = time.perf_counter()
    model = TextEmbedding(model_name="BAAI/bge-small-en-v1.5")
    load_ms = int((time.perf_counter() - t0) * 1000)
    print(f"model_load_ms={load_ms}", file=sys.stderr)

    # Warm up
    list(model.embed(CORPUS))

    t1 = time.perf_counter()
    embeddings = list(model.embed(CORPUS))
    embed_us = int((time.perf_counter() - t1) * 1_000_000)
    n = len(embeddings)
    throughput = n / (embed_us / 1_000_000)
    print(
        f"n={n} embed_us={embed_us} throughput_texts_per_sec={throughput:.1f}",
        file=sys.stderr,
    )

    t2 = time.perf_counter()
    for t in CORPUS:
        list(model.embed([t]))
    single_us = int((time.perf_counter() - t2) * 1_000_000)
    print(f"single_text_avg_us={single_us // n}", file=sys.stderr)

    out = [
        {"text": t, "embedding": e.tolist()}
        for t, e in zip(CORPUS, embeddings, strict=True)
    ]
    print(json.dumps(out))


if __name__ == "__main__":
    main()
