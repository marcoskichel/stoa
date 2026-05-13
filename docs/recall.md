# Recall

All retrieval in Stoa goes through the `stoa-recalld` daemon's `search` RPC. The daemon delegates to MemPalace's `searcher.search_memories`, which runs a hybrid pipeline:

1. **Cosine retrieval.** ChromaDB pulls the top `n_results * 3` drawers by HNSW cosine distance.
2. **BM25 rerank.** Okapi-BM25 is computed over the candidate pool against the query and used to reorder the top results.
3. **Optional union widen.** With `candidate_strategy="union"`, MemPalace pulls additional candidates from its SQLite FTS5 index and merges them into the rerank pool. Stoa does not enable this by default.
4. **Closet boost.** MemPalace's "closet" entries (per-source topic summaries) contribute a rank-based boost to drawers in matching sources.

Result: each hit carries `score` (cosine similarity, `max(0, 1 - distance)`), `snippet` (the matched chunk text, expanded with ±1 sibling chunks when available), `source_path`, and `metadata` (including `kind`, `wing`, `room`).

## Filters

Stoa adds one metadata convention on top of MemPalace's `wing` / `room` namespacing:

| Filter | Meaning |
|---|---|
| `kind=wiki` | Only curated wiki pages (default for `stoa query` and `stoa-inject-hook`). |
| (no `kind` filter) | All drawers — verbatim conversation chunks + wiki pages. |

`stoa query "..."` defaults to `kind=wiki`; pass `--include-drawers` to drop the filter:

```bash
stoa query "session token TTL"                  # wiki only
stoa query "session token TTL" --include-drawers # wiki + drawers
```

## Wiki-as-drawer

`stoa write` writes the wiki page to disk AND inserts the same content as a drawer in the MemPalace palace with metadata:

```json
{
  "kind": "wiki",
  "wiki_kind": "entity",
  "wiki_id": "ent-redis",
  "source_file": "wiki/entities/ent-redis.md",
  "wing": "__stoa_wiki__",
  "room": "entity",
  "title": "Redis"
}
```

The `wing=__stoa_wiki__` separates wiki pages from conversation drawers in MemPalace's wing-based view. The `kind=wiki` metadata is what `stoa query` and the inject hook filter on.

## CLI

```bash
stoa query "redis caching"                # top 5 wiki hits
stoa query "redis caching" --top-k 10     # top 10 wiki hits
stoa query "redis caching" --include-drawers   # also include conversation drawers
```

Output:

```
1. [score=0.870] wiki/entities/ent-redis.md
   In-memory data store. Used for caching session tokens and rate limiting.
2. [score=0.712] wiki/concepts/con-cache-keys.md
   Cache key shape across services.
...
```

## The inject path

`stoa-inject-hook` builds a query from:

- **For `UserPromptSubmit`**: primarily the user's prompt text, with workspace signals (cwd basename, git remote, recently-edited wiki page stems + H1 titles) as a fallback ladder.
- **For `SessionStart`**: workspace signals only (no prompt yet) — full signal joined first, then progressively narrower fallbacks down to "first wiki stem token alone".

The first query in the ladder that produces non-empty hits wins. The hit set is wrapped in the MINJA-resistant envelope and emitted as `additionalContext`. See [injection.md](injection.md) for the envelope details.

## Score interpretation

The `score` returned by the daemon is `max(0, 1 - cosine_distance)`. Practical ranges:

| Score | Meaning |
|---|---|
| ≥ 0.85 | Very strong semantic match. |
| 0.65 – 0.85 | Plausible match; usually keep. |
| 0.45 – 0.65 | Weak match; useful only if the topic is sparse. |
| < 0.45 | Probably noise. |

The inject hook's relevance gate accepts any hit with `score > 0` — the top hit's score is what determines whether the envelope is emitted at all (no top hit above floor → empty injection).

## Backends

`stoa-recall` defines a `RecallBackend` trait. v0.1 ships only `MempalaceBackend` (Unix-socket client). The trait stays for the day a better substrate appears — backend swaps require zero changes to the hooks or CLI.

## Next

- [Injection](injection.md) — how the daemon's hits are wrapped and surfaced.
- [Troubleshooting](troubleshooting.md) — empty results, daemon timeouts, etc.
