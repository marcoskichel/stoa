# Capture pipeline

Capture is the path from "the agent finished a session" to "a redacted
JSONL transcript is on disk under `sessions/`". The hot path runs in
under 10 ms p95; everything else happens in workers.

## The 10 ms budget

```
Claude Code  ──fires Stop hook──►  stoa-hook binary
                                        │
                                        │  open .stoa/queue.db (WAL)
                                        │  INSERT one row
                                        │  exit 0
                                        ▼
                                  return control to agent
```

That is the entire hot path. `stoa-hook` does **not**:

- run any redaction
- write to `sessions/`
- update any index
- spawn any subprocess

It is a single static binary that opens a SQLite database in WAL mode
(`synchronous=NORMAL`), inserts one row, and exits. The latency budget
exists because anything heavier means the agent UI feels stuck on
session end. The CI gate on `rust.yml` enforces it for every PR.

## The queue

The queue is `.stoa/queue.db` — a SQLite database with WAL +
`synchronous=NORMAL`. WAL gives concurrent reader / single writer
without blocking; `synchronous=NORMAL` trades the tail of fsync calls
for the durability guarantee we need (no committed row is ever lost
across a crash). Each queued row carries:

- the agent + session id
- the path to the transcript file (Claude Code writes its own jsonl
  alongside the session)
- a claim lease so a worker crash mid-processing leaves the row
  unprocessed instead of lost.

## The capture worker

The worker runs inside `stoa daemon`. It loops:

1. Claim the next unprocessed row (atomic update with a lease).
2. Read the transcript file referenced by the row.
3. Run PII redaction (see below).
4. Append the redacted JSONL to `sessions/<session-id>.jsonl`.
5. Append a `stoa.capture` event to `.stoa/audit.log`.
6. Mark the queue row done.

Crash recovery is automatic — a claim lease that expires returns the
row to the queue and the next worker picks it up. The result is
**always-flush**: any session that fires its `Stop` hook ends up on
disk, regardless of session length, network state, or whether the
daemon was running at the moment of capture.

## PII redaction

The redactor runs a fixed set of regex patterns in `crates/stoa-capture`:

- AWS access keys
- Stripe live + test keys
- OpenAI keys
- Anthropic keys
- GitHub Personal Access Tokens (classic + fine-grained)
- `Bearer` tokens
- JWTs
- Email addresses
- SSH / AWS / GPG path patterns (`~/.ssh/id_*`, `~/.aws/credentials`,
  etc.)

Matched substrings are replaced with placeholders like
`<REDACTED:aws-access-key>`. The patterns are intentionally fixed for
v0.1 — runtime-configurable patterns ship in v0.2.

!!! warning "Pre-redaction transcripts"
    The transcript file the agent writes is pre-redaction. Only the
    JSONL under `sessions/` has the redactor applied. Never paste a raw
    Claude Code transcript into an issue without checking it manually.

## Install + uninstall

Register the hook:

```bash
stoa hook install --platform claude-code
```

This writes a `Stop` / `SessionEnd` hook entry into Claude Code's
config pointing at the absolute path of the `stoa-hook` binary.

Remove the hook:

```bash
stoa hook uninstall --platform claude-code
```

## Audit trail

Every capture event is appended to `.stoa/audit.log` as a single JSON
line:

```json
{"ts":"2026-05-13T01:12:34Z","event":"stoa.capture","session_id":"01JC...","bytes":4821}
```

The log is append-only; the daemon never rewrites prior entries.

## Next

- [Recall](recall.md) — how captured sessions feed the index.
- [SessionStart injection](injection.md) — the other side of the
  agent-hook loop.
