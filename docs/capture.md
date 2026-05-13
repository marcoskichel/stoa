# Capture pipeline

Capture in v0.1 is **delegated to MemPalace**. Stoa's contribution is the <10 ms Rust hook that fires the `mine` RPC; everything past that is MemPalace.

## The hook

`stoa-hook` is the binary Claude Code invokes on `Stop` / `SessionEnd`. It:

1. Reads the Claude Code hook payload on stdin (max 256 KiB).
2. Extracts `transcript_path` from the JSON.
3. Connects to `$XDG_RUNTIME_DIR/stoa-recalld.sock`.
4. Sends `{"method":"mine","params":{"source_file":"<transcript_path>","session_id":"<id>"}}`.
5. Closes the connection and exits 0.

Cold-start budget: <10 ms p95. The hook uses `std::os::unix::net::UnixStream` (no async runtime), bounds stdin to 256 KiB, and does best-effort error handling — if the daemon is missing or the socket is unreachable, the hook still exits 0 so a down daemon never breaks the agent loop.

## What MemPalace does with the transcript

The `mine` RPC routes to MemPalace's `miner.mine_file(source_file=…, palace_path=…)`. MemPalace:

- Splits the transcript into drawers (~paragraph-sized verbatim chunks).
- Computes embeddings with its default sentence-transformer model (CPU, ~300 MB on disk).
- Stores drawers in ChromaDB with cosine distance.
- Adds BM25 sparse representations to its SQLite FTS5 index.
- Indexes any entities it detects into the knowledge graph (separate SQLite db).

The drawer metadata carries `source_file`, `chunk_index`, `wing` (project), `room` (aspect). Stoa does not override these; MemPalace's miner sets them.

## PII redaction

MemPalace exposes a `sanitize_query` helper for query-side defense and a configurable mining pipeline. In v0.1 Stoa does NOT layer additional redaction on top of MemPalace's defaults; that's tracked as a v0.1.x follow-up (the previous Rust `stoa-capture` regex pipeline was deleted in the pivot — see [docs/adr/0001-mempalace-pivot.md](adr/0001-mempalace-pivot.md)).

If your sessions contain secrets your workflow needs scrubbed BEFORE they hit MemPalace, run a pre-mine filter on the transcript yourself, point `stoa-hook` at the filtered copy via a wrapper script, or use Claude Code's built-in transcript redaction features.

## What gets indexed

| What | Where |
|---|---|
| Verbatim conversation drawers | MemPalace palace (`.stoa/palace/`) |
| Wiki pages tagged `kind=wiki` | Same palace, different metadata |
| Raw URLs, PDFs, etc. you ingest | Drop them under `raw/`, run `mempalace mine raw/` directly |

The wiki and the drawer corpus share **one retrieval index** — Stoa filters by `kind=wiki` when the agent wants curated context, falls through to drawers when wiki coverage is thin.

## Audit

Capture does not append to `.stoa/audit.log` — only injection does. To inspect what MemPalace stored, use:

```bash
stoa query "<some text from your last session>" --include-drawers
mempalace search "<text>"     # MemPalace's CLI, equivalent surface
```

## Failure modes

- **Daemon down.** `stoa-hook` exits 0, the session is NOT indexed, no audit row is written. Restart with `stoa daemon start`.
- **MemPalace not installed.** The daemon will fail health check at startup; restart the daemon after `uv tool install mempalace`.
- **Transcript path missing.** `stoa-hook` exits 0 silently (no transcript = nothing to mine).
- **Oversize stdin.** Stdin is bounded to 256 KiB; oversize payloads degrade to a default hook exit (no mine call).

## Next

- [Recall](recall.md) — how `stoa query` and the inject hook surface what MemPalace stored.
- [Injection](injection.md) — how wiki hits land in front of the agent.
