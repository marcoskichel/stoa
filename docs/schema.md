# Wiki schema

The schema is a markdown document at the workspace root called `STOA.md`. It declares the entity types and relationship types your wiki accepts. Pages that violate the schema fail `stoa schema --check`.

## Layout

```
STOA.md                     # workspace schema
wiki/
  entities/                 # one file per "thing" with identity
    ent-redis.md
    ent-acme-billing.md
  concepts/                 # one file per abstract idea
    con-cache-keys.md
  synthesis/                # crystallized notes (auto, see stoa-crystallize)
raw/                        # ingested external content
sessions/                   # redacted session transcripts (MemPalace-mined)
```

The `entities/` / `concepts/` / `synthesis/` split is the [Karpathy LLM Wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) pattern. `entities/` are nouns with identity. `concepts/` are ideas. `synthesis/` is for cross-page distillations produced by `stoa-crystallize`.

## Page anatomy

Every wiki page is a markdown file with YAML frontmatter:

```markdown
---
id: ent-redis
title: Redis
status: active
kind: entity
type: library
created: 2026-05-12T00:00:00Z
updated: 2026-05-13T00:00:00Z
relationships:
  - type: uses
    target: ent-acme-cache
---

# Redis

In-memory data store. Used for caching session tokens and rate limiting.

## What we decided

- 7-day TTL for refresh tokens.
- Cluster mode in prod, single instance in dev.
```

Required on every page:

- `id` — globally unique, kebab-case, prefixed by kind (`ent-`, `con-`, `syn-`).
- `title` — human-readable display name.
- `status` — `active`, `superseded`, `stale`, or `deprecated`.
- `kind` — `entity`, `concept`, or `synthesis`.
- `created`, `updated` — RFC 3339 / ISO 8601 timestamps.

Required for `kind: entity`:

- `type` — must appear in `STOA.md`'s entity-type allow-list.

Optional:

- `relationships` — list of `{type, target}` pairs. `type` must appear in the relationship-types allow-list; `target` must be a valid page id.
- For `kind: synthesis`: `inputs:` (list of source page ids) and `question:` (the cross-page question this page answers).

## Writing pages

`stoa write` is the **only** write path. Direct edits to `wiki/*.md` survive in source control but the MemPalace index will not see them until you re-write the page through the CLI.

```bash
stoa write ent-redis \
  --frontmatter /tmp/redis-fm.yaml \
  --body /tmp/redis-body.md

stoa read ent-redis
```

`stoa write` is idempotent on `page_id`. The daemon upserts both the markdown file and the MemPalace drawer.

## Validation

```bash
stoa schema           # print the active STOA.md
stoa schema --check   # walk wiki/, validate each page
```

The check parses each page's frontmatter and runs `stoa_core::validate_page` against the schema parsed out of `STOA.md`. Violations print one per line and the command exits non-zero.

## STOA.md format

`stoa init` ships a starter `STOA.md` shaped like:

```markdown
# Entity types
- `library` — third-party code you depend on
- `service` — running process or daemon
- `tool` — CLI or developer-facing utility
- `team` — group of people
- `concept` — domain idea or abstraction

# Relationship types
- `uses`
- `depends_on`
- `related_to`
- `supersedes`
```

`Schema::from_stoa_md` scans for bullet items under `# Entity types` / `# Relationship types` headings, taking the first backtick-quoted token (or first whitespace-separated word) as the vocabulary entry. The defaults (`library`, `service`, etc.) are always merged in; anything new you add **extends** the allow-list.

## Next

- [Capture pipeline](capture.md) — how `sessions/` are populated by MemPalace.
- [Recall](recall.md) — how `wiki/` content is indexed for retrieval.
