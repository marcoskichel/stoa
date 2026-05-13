# Recall

Recall is the retrieval layer: `stoa query` and the SessionStart hook
both call into the same `RecallBackend` trait to find relevant content.

## The trait

```rust
pub trait RecallBackend {
    fn search(&self, query: &str, k: usize) -> Result<Vec<Hit>>;
    fn index(&self, doc: &Document) -> Result<()>;
    fn delete(&self, id: &Id) -> Result<()>;
}

pub struct Hit {
    pub id: Id,
    pub source_path: PathBuf,
    pub score: f32,
    pub streams: Vec<Stream>,    // which streams matched
}
```

Every recall implementation produces `Hit`s with `source_path` always
resolving to a real file on disk. That guarantee makes Stoa's output
verifiable — the agent can quote by path and a human can open it.

## The default backend: `LocalChromaSqliteBackend`

`LocalChromaSqliteBackend` is the v0.1 default. It runs three streams
in parallel and fuses with **reciprocal rank fusion** (RRF, k=60):

```
                        ┌──────────────────────┐
   query  ─────────────►│  BM25 (SQLite FTS5)   │──┐
                        └──────────────────────┘  │
                        ┌──────────────────────┐  │
                ───────►│  Vector (ChromaDB)    │──┼──► RRF fusion ──► ranked hits
                        └──────────────────────┘  │
                        ┌──────────────────────┐  │
                ───────►│  Graph (typed KG)     │──┘
                        └──────────────────────┘
```

- **BM25** lives in `.stoa/recall.db` (SQLite FTS5 in the same file as
  the queue, separate tables).
- **Vector** uses ChromaDB embeddings (`bge-small-en-v1.5` by default).
  Stored under `.stoa/vectors/`.
- **Graph** is two SQLite tables (`nodes`, `edges`) holding the typed
  knowledge graph derived from frontmatter `relationships`.

RRF is unweighted by default — each stream contributes equally. Per-hit
provenance carries which streams matched, so downstream consumers can
re-weight if needed.

## `stoa query`

```bash
stoa query "redis vs memcached"
stoa query "redis vs memcached" --k 20
stoa query "redis vs memcached" --streams bm25,vector --json
```

Flags:

- `--k <N>` — number of hits to return (default 10).
- `--streams <a,b,c>` — restrict to a subset of the three streams.
  Useful for debugging which stream surfaced a result.
- `--json` — emit one JSON object per line for downstream tooling.

## Indexing

Indexing happens two ways:

1. **Live.** The daemon watches `wiki/` for changes (`notify` crate)
   and re-indexes any page whose mtime changes. New session JSONL files
   are also indexed when the capture worker writes them.
2. **Rebuild.** `stoa index rebuild` (or `stoa rebuild`) tears down
   `.stoa/recall.db` and `.stoa/vectors/` and rebuilds them from the
   source-of-truth files under `wiki/`, `raw/`, and `sessions/`. This
   is the load-bearing invariant: nothing lives only in the index.

## Cold-start budget

| Operation                              | Budget         |
| -------------------------------------- | -------------- |
| `stoa init --no-embeddings`            | <5s            |
| `stoa init` (with embedding model dl)  | <60s           |
| Single `stoa query` after warm-up      | <100ms p95     |
| `stoa rebuild` over 1000-page wiki     | seconds-scale  |

`--no-embeddings` is the BM25-only mode for environments that cannot
fetch the embedding model.

## Benchmarks

Recall accuracy is measured against the v0.1 benchmark suite:

- **LongMemEval** — long-context dialog memory recall
- **MemoryAgentBench** — multi-turn agent memory tasks
- **MEMTRACK** — memory tracking + redundancy metrics
- **BEAM** — embedding retrieval at scale
- **AgentLeak** — adversarial info-leakage probes

Numbers are published per-backend under
`benchmarks/results/v0.1-<backend>-<benchmark>.md` — see
[benchmarks/README.md](https://github.com/marcoskichel/stoa/blob/main/benchmarks/README.md).

## Swappable backends

The `RecallBackend` trait is the seam for community adapters. Future
backends (Qdrant, LanceDB, pure-Rust embedding stack) implement the
same three-method interface; nothing in the rest of Stoa depends on
the concrete backend.

## Next

- [SessionStart injection](injection.md) — how query results become
  context.
- [Troubleshooting](troubleshooting.md) — empty results, missing
  embeddings, daemon-not-running symptoms.
