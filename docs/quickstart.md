# Quickstart

Get from a fresh project directory to your first injected `<stoa-memory>`
block in roughly five minutes.

## 1. Scaffold a workspace

```bash
cd ~/projects/your-project
stoa init
```

`stoa init` is idempotent — running it twice does not clobber existing
state. It creates:

```
STOA.md           # schema definition (entity types, relationships)
wiki/
  entities/       # one .md file per concrete thing (Redis, your-team)
  concepts/       # one .md file per abstract idea (caching, ADRs)
  synthesis/      # crystallized cross-page notes (auto-generated v0.2+)
raw/              # ingested URLs, PDFs, plain text
sessions/         # redacted JSONL transcripts (one per agent session)
.stoa/
  queue.db        # SQLite WAL queue (hook → worker)
  recall.db       # BM25 + KG index (rebuildable)
  vectors/        # embedding store (rebuildable)
  audit.log       # append-only event log
.gitignore
```

See [Wiki schema](schema.md) for the file format and frontmatter rules.

## 2. Register the agent hooks

For Claude Code:

```bash
stoa hook install --platform claude-code --inject session-start
```

This registers two hooks with your local Claude Code config:

- **`Stop` / `SessionEnd`** — fires when a session ends, invoking
  `stoa-hook` to enqueue the transcript.
- **`SessionStart`** — fires at the top of every new session, invoking
  `stoa-inject-hook` to emit the relevant wiki pages as
  `additionalContext`.

See [Capture pipeline](capture.md) and
[SessionStart injection](injection.md) for what each hook does.

## 3. Start the background worker

```bash
stoa daemon &
```

The daemon drains `.stoa/queue.db`: it runs PII redaction, writes
session JSONL files, indexes new wiki pages, and (in v0.2+) runs the
harvest / crystallize loops.

The capture hooks themselves do not depend on the daemon being live —
they enqueue and exit in <10 ms. The daemon catches up whenever it is
running.

## 4. Use your agent normally

Open a new session in Claude Code. At the top of the session prompt
Stoa injects a `<stoa-memory>` block with relevant wiki pages (if any
exist yet). When the session ends, the transcript lands in
`sessions/<session-id>.jsonl` with API keys and tokens redacted.

After a few sessions:

```console
$ stoa query "what did we decide about caching"
wiki/entities/ent-redis.md   score=4.21
wiki/concepts/cache-keys.md  score=2.87
sessions/01JC.../jsonl       score=2.04
```

`stoa query` runs a hybrid search across the wiki and session
transcripts (vector + BM25 + KG via reciprocal rank fusion). See
[Recall](recall.md).

## 5. Inspect what was injected

```bash
stoa inject log
```

Prints the most recent injection events from `.stoa/audit.log` —
session id, query used, hit count, characters injected, and the full
wrapped block. This is your audit trail for what landed in the agent's
prompt and why.

## Next

- [Wiki schema](schema.md) — how to add structured entities by hand.
- [Recall](recall.md) — fusion behavior and per-stream provenance.
- [Troubleshooting](troubleshooting.md) — common first-run failures.
