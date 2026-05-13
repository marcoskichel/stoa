# Wiki schema

The schema is a YAML document at the repo root called `STOA.md`. It
declares the **entity types**, **relationship types**, and **required
frontmatter fields** that the wiki accepts. Frontmatter is parsed by
`serde_yaml`; pages that violate the schema fail `stoa schema --check`
and are not indexed.

## Layout

```
STOA.md                     # the schema
wiki/
  entities/                 # one file per "thing" with identity
    ent-redis.md
    ent-acme-billing.md
  concepts/                 # one file per abstract idea
    con-cache-keys.md
  synthesis/                # crystallized notes (auto, v0.2+)
raw/                        # ingested external content
sessions/                   # redacted session transcripts
```

The `entities/` / `concepts/` / `synthesis/` split is the
[Karpathy LLM Wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
pattern. `entities/` are nouns with identity. `concepts/` are ideas.
`synthesis/` is for cross-page distillations.

## Page anatomy

Every wiki page is a markdown file with YAML frontmatter:

```markdown
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-13
relationships:
  - type: uses
    target: ent-acme-cache
---

# Redis

In-memory data store. Used for caching session tokens and rate limiting.

## What we decided

- 7-day TTL for refresh tokens.
- Cluster mode in prod, single instance in dev.

## Sources

- sessions/01JC...
```

Required fields:

- `id` — globally unique, kebab-case, prefixed by kind (`ent-`,
  `con-`, `syn-`).
- `kind` — one of `entity`, `concept`, `synthesis`.
- `type` — must be drawn from `STOA.md`'s `entity_types` /
  `concept_types` vocabulary.
- `created`, `updated` — ISO 8601 dates.

Optional:

- `relationships` — list of `{type, target}` pairs; `type` must be one
  of `STOA.md`'s `relationship_types`; `target` must be a valid `id`.

## Editing pages

Manual writes via the CLI:

```bash
# Create or update from disk fragments
stoa write ent-redis \
    --frontmatter /tmp/redis-fm.yaml \
    --body /tmp/redis-body.md

# Read it back
stoa read ent-redis
```

`stoa write` is idempotent: re-writing an existing id replaces the
file atomically (write-temp + rename). The `updated` field is bumped
automatically.

## Validation

```bash
stoa schema           # print the active STOA.md
stoa schema --check   # fail on any schema violation in wiki/
```

`stoa schema --check` is what CI hooks in for "wiki health" gates.

## index.md and log.md

Both files are **auto-generated** under `wiki/` — never edit them by
hand. They are rewritten whenever the daemon (or `stoa rebuild`) sees a
new wiki page.

## Default schema

`stoa init` ships a starter `STOA.md` with sensible defaults for
software projects (`library`, `service`, `tool`, `team`, `concept`
types; `uses`, `depends_on`, `related_to`, `supersedes` relationships).
Edit it freely — Stoa re-reads `STOA.md` on every command.

## Next

- [Capture pipeline](capture.md) — how `sessions/` files are
  produced.
- [Recall](recall.md) — how `wiki/` content is indexed for retrieval.
