# Quickstart

Five commands from zero to a Stoa workspace that injects wiki hits into Claude Code.

## 1. Initialize a workspace

```bash
cd ~/projects/myapp
stoa init
```

Scaffolds `STOA.md`, `wiki/{entities,concepts,synthesis}/`, `raw/`, `sessions/`, `.stoa/`, and a `.gitignore` for `.stoa/`.

## 2. Start the daemon

```bash
stoa daemon start
stoa daemon status    # confirm it's up
```

The daemon walks up from your `$PWD` until it finds `STOA.md`, then binds a Unix socket and mounts MemPalace at `.stoa/palace/`. If you're outside a workspace, `stoa daemon start` will exit non-zero.

## 3. Install Claude Code hooks

```bash
stoa hook install --platform claude-code --inject
```

Prints a JSON snippet you paste into `~/.claude/settings.json` under the `hooks` key. Stoa never edits agent settings — you decide what gets installed.

The default snippet wires four hooks: `Stop` and `SessionEnd` → `stoa-hook` (capture), and `SessionStart` + `UserPromptSubmit` → `stoa-inject-hook` (injection). Drop `--inject` if you only want capture without per-prompt injection.

## 4. Write a wiki page

```bash
cat >/tmp/redis-fm.yaml <<EOF
id: ent-redis
title: Redis
status: active
kind: entity
type: library
created: 2026-05-13T00:00:00Z
updated: 2026-05-13T00:00:00Z
EOF
cat >/tmp/redis-body.md <<'EOF'
In-memory data store. Used for caching session tokens and rate limiting.

## What we decided
- 7-day TTL for refresh tokens.
- Cluster mode in prod, single instance in dev.
EOF
stoa write ent-redis --frontmatter /tmp/redis-fm.yaml --body /tmp/redis-body.md
```

`stoa write` does TWO things atomically:

1. Writes `wiki/entities/ent-redis.md` to disk.
2. Upserts the same content as a `kind=wiki` drawer in MemPalace via the daemon's `write_wiki` RPC.

## 5. Search it

```bash
stoa query "redis caching" --top-k 5
```

You should see `wiki/entities/ent-redis.md` with a relevance score. The same daemon path the inject hook uses.

## What happens next

When you next start a Claude Code session in this workspace:

- `SessionEnd`/`Stop` → `stoa-hook` fires `mine` on the transcript; MemPalace indexes every turn.
- `UserPromptSubmit` → `stoa-inject-hook` queries the daemon for matching wiki hits, wraps them in `<stoa-memory>`, and prepends them to your prompt as `additionalContext`.
- `stoa inject log --limit 5` shows you exactly what was injected.

## Optional: harvest

```bash
export ANTHROPIC_API_KEY=...
stoa-harvest run --query "decisions" --top-k 20
```

Pulls 20 recent drawers from MemPalace, asks Claude to identify durable entities + decisions, writes the resulting wiki page candidates back through the daemon. Without an API key the command is a no-op (it prints why and exits 0).

## Optional: crystallize

```bash
stoa-crystallize run "why did we switch to GraphQL"
```

Pulls relevant wiki pages, asks Claude to synthesize a cross-page answer, writes a `kind: synthesis` page citing its sources in the `inputs:` frontmatter.

## Next

- [Wiki schema](schema.md) — what every page needs.
- [Recall](recall.md) — how the daemon ranks results.
- [Injection](injection.md) — the MINJA-safe envelope wrapping.
