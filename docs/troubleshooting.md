# Troubleshooting

## `stoa daemon status` says "daemon unavailable"

The daemon isn't running. Start it:

```bash
stoa daemon start
sleep 1
stoa daemon status
```

If the daemon is supposedly running but the socket is unreachable, check the PID file and log:

```bash
cat $XDG_RUNTIME_DIR/stoa-recalld.pid          # or /tmp/stoa-recalld-$USER.pid
tail -50 ~/.local/state/stoa/recalld.log       # or $STOA_RECALLD_LOG_FILE
```

If the PID file exists but the process is gone, delete the PID file and start fresh.

## `stoa daemon start` fails with "No STOA.md found"

The daemon binds to the workspace containing `STOA.md`. If you're outside a workspace, scaffold one first:

```bash
cd ~/projects/myapp
stoa init
stoa daemon start
```

## `stoa daemon start` fails with "mempalace package not installed"

Install MemPalace:

```bash
uv tool install mempalace
# or
pip install mempalace
```

The daemon imports `mempalace` lazily but will surface a clear error in the health response.

## Empty injections in Claude Code sessions

Run:

```bash
stoa inject log --limit 5
```

Each row has `hits` (count) and `chars_injected`. If both are zero across every recent session:

- **Wiki is empty.** Write some pages with `stoa write`, or run `stoa-harvest run`.
- **Query produces no matches.** Check what the hook is asking: the `query` field in the audit log shows the effective query. Try the same query manually with `stoa query "..."`.
- **Daemon is unreachable.** `stoa daemon status` should respond.

## `stoa inject log` shows hits but the agent doesn't act on them

Check that the snippet content is what you expect — the agent sees the literal `additional_context` field. Common causes:

- Wiki pages are very short and contain only frontmatter / placeholders. The snippet body is just the markdown body; if the body is `(no excerpt)`, the agent has nothing to anchor on.
- The wiki page title doesn't appear in the body. The H1 (`# Redis`) is often what gives the model enough surface to act on.

## `stoa write` fails with "frontmatter.kind must be one of entity|concept|synthesis"

Your frontmatter YAML is missing the `kind:` field or has a value Stoa doesn't accept. Required values: `entity`, `concept`, `synthesis`. See [schema.md](schema.md) for the full required-fields list.

## `stoa schema --check` reports "missing required field `type` for entity"

Entity pages MUST carry a `type:` field. Add it:

```yaml
---
id: ent-redis
title: Redis
kind: entity
type: library    # ← this line
status: active
created: ...
updated: ...
---
```

The `type` value must be in the schema's entity-types allow-list (defaults: `library`, `service`, `tool`, `team`, `concept`).

## Hooks fire but `additionalContext` never appears in the agent's view

Claude Code only honors the `hookSpecificOutput` shape with `hookEventName` matching the actual event. Check the snippet in `~/.claude/settings.json` matches the snippet `stoa hook install --inject` prints. Common mistake: only wiring `SessionStart` and expecting per-prompt injection — `UserPromptSubmit` is the per-prompt path.

## Cold-start latency on the first prompt feels long

Realistic warm-up is ~400-800 ms — MemPalace loads its HNSW segment lazily on the first query. Mitigations:

- Start the daemon before launching Claude Code: `stoa daemon start` in a terminal first.
- Wait until `stoa daemon status` returns a `mempalace_version` before opening a session.

## "I edited a wiki page by hand; `stoa query` doesn't find it"

Hand edits skip the daemon's `write_wiki` RPC, so the MemPalace index never sees them. Either:

- Re-write the page through `stoa write` (idempotent).
- Run `stoa-harvest run` — the harvest worker re-pulls drawers and emits write_wiki calls, but the page won't appear unless harvest's LLM picks it up.

A future `stoa index rebuild` will let you re-mirror `wiki/*.md` into MemPalace without using the LLM. Tracked in [ROADMAP.md](https://github.com/marcoskichel/stoa/blob/main/ROADMAP.md) §M-v0.1.x.

## Audit log keeps reporting empty injections

Means the hook is wired correctly but the daemon returns no hits. Cross-check with `stoa query` for the same effective query (from the audit row). If `stoa query` returns hits but the audit log shows zero, file a bug — the inject hook's query ladder may be filtering harder than expected.

## MemPalace warns "this palace was created without cosine distance"

A legacy palace was created with the default L2 metric. Fix:

```bash
mempalace repair
```

Then restart the daemon: `stoa daemon stop && stoa daemon start`. Stoa initializes palaces with cosine distance for new workspaces, so this only affects palaces predating the pivot.

## Where to get help

- File an issue: https://github.com/marcoskichel/stoa/issues
- Discord (MemPalace's): linked from MemPalace's README; relevant for MemPalace-specific questions.
