# Stoa

> The painted porch for AI memory.

Stoa is a **Rust hook surface + curated LLM wiki** layered over [MemPalace](https://github.com/MemPalace/mempalace). MemPalace stores every conversation verbatim and serves hybrid BM25 + cosine search; Stoa curates a wiki of entities, concepts, and synthesis pages on top, and injects relevant wiki hits at every user prompt through Claude Code's `UserPromptSubmit` hook.

## The flow

```
┌───────────────────────────────────────────────────┐
│ You type a prompt                                 │
└──────────────────────┬────────────────────────────┘
                       ▼
        Claude Code fires UserPromptSubmit
                       │
                       ▼
        stoa-inject-hook (Rust, <500 ms warm)
                       │ JSON-line over $XDG_RUNTIME_DIR/stoa-recalld.sock
                       ▼
        stoa-recalld (Python daemon, hosts MemPalace)
                       │
                       ▼
        Top-K wiki hits with relevance scores
                       │
                       ▼
        Wrapped in <stoa-memory> envelope
        (preamble + provenance + MINJA-safe)
                       │
                       ▼
   additionalContext appended to your prompt
                       │
                       ▼
        Agent answers with the wiki in front of it
```

## Two non-negotiable patterns

1. **Wiki on disk is canonical.** Every wiki page is a markdown file under `wiki/`. `stoa-recalld` mirrors them into MemPalace tagged `kind=wiki`, but the file is the source of truth. Delete `.stoa/`, replay your `stoa write`s (or `stoa-harvest run`), and the index regenerates.
2. **Hook → daemon RPC.** Rust hooks are <10 ms (`stoa-hook` for `SessionEnd`) or <500 ms warm (`stoa-inject-hook` for `SessionStart` + `UserPromptSubmit`). Both shoot a single JSON line at the daemon's Unix socket and exit. All heavy work lives in the daemon.

## What's in v0.1

- `stoa` CLI for workspace + wiki + daemon orchestration.
- `stoa-hook` + `stoa-inject-hook` Rust binaries that talk to the daemon.
- `stoa-recalld` Python daemon hosting MemPalace.
- `stoa-harvest` + `stoa-crystallize` LLM workers (Anthropic-backed; no-op without an API key).
- MINJA-resistant `<stoa-memory>` envelope with per-injection audit log.

## Where to start

- [Install](install.md) — get the binary + daemon on your machine.
- [Quickstart](quickstart.md) — `stoa init` → `stoa daemon start` → `stoa write` → `stoa query` in five commands.
- [Wiki schema](schema.md) — what `STOA.md` controls and what every wiki page needs.
- [Capture pipeline](capture.md) — how `SessionEnd` becomes drawers in MemPalace.
- [Recall](recall.md) — how `stoa query` and the inject hook fetch wiki hits.
- [Injection](injection.md) — what `UserPromptSubmit` puts in front of the agent.
- [Troubleshooting](troubleshooting.md) — daemon down, empty injection, missing MemPalace.

See also: [ARCHITECTURE.md](https://github.com/marcoskichel/stoa/blob/main/ARCHITECTURE.md) is the authoritative source of truth. [docs/adr/0001-mempalace-pivot.md](adr/0001-mempalace-pivot.md) records why Stoa wraps MemPalace.
