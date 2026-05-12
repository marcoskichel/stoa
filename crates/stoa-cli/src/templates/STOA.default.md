# STOA.md — Workspace schema

This file is the workspace's instruction manual. Every agent that touches
the wiki loads this schema into its context. Edit it to encode your
domain; commit it alongside the wiki.

Spec source: [ARCHITECTURE.md §3 Schema] + [ARCHITECTURE.md §5 KG].

## Entity types

A wiki entity is "a real thing with identity over time". The following
types ship with Stoa by default — extend the list as your workspace grows.

- `person` — Authors, teammates, contacts.
- `project` — Repositories, products, internal initiatives.
- `library` — Packages, frameworks, runtime dependencies.
- `service` — APIs, hosted services, infrastructure components.
- `tool` — CLIs, binaries, editor plugins.
- `file` — Specific source files of interest.
- `decision` — Recorded choices, ADRs, design notes.
- `concept` — Abstract topics (mirrors the `concept` page kind).

## Relationship types

Typed edges between pages. Frontmatter `relationships` entries must use one
of these `type:` values; `stoa schema --check` rejects unknown ones.

- `uses` — A consumes B at runtime.
- `depends_on` — A requires B to function.
- `instance_of` — A is a specific kind of B.
- `caused` — A produced B (incident or decision chain).
- `fixed` — A resolved B.
- `supersedes` — A replaces B (B's status becomes `superseded`).
- `contradicts` — A and B make incompatible claims.
- `cites` — A references B as a source.
- `mentions` — A names B without a strong relation.

## Ingest rules

How to handle each source kind when content lands in `raw/`:

- **PDFs / papers** — extract sections, cite the file under `raw/`.
- **URLs / web pages** — fetch + summarize; keep the fetch sidecar.
- **Chat transcripts** — extract decisions, entities, open questions.
- **Code / source files** — link by path; do not duplicate content.

## Page creation rules

- Prefer linking to an existing entity over creating a new one.
- Create a new entity when no existing entity matches by alias or fuzzy
  title; flag ambiguous cases for human review.
- Use `synthesis` pages for cross-cutting essays, never as long-form
  entity dumps.

## Quality bar

- Every page must have a non-empty `title:`.
- Pages should cite at least one source from `raw/` when possible.
- Synthesis pages must list `inputs:` (the upstream entities + concepts +
  raw artifacts they were built from).

## Contradiction policy

- Surface contradictions explicitly via the `contradicts` relationship.
- New consensus supersedes the old via `supersedes` (status: `superseded`).
- Stoa does not silently delete superseded pages.

## Consolidation schedule

- Crystallization runs on demand (see ARCHITECTURE §9.2). The defaults are
  intentionally conservative — change only after measuring.

## Privacy redactions

- The capture pipeline (ARCHITECTURE §7) ships sensible default PII
  patterns. Add domain-specific patterns here as bullet points; the
  capture worker will pick them up on the next pass.

## Scoping

- `wiki/` is shared knowledge — visible to every agent in this workspace.
- `sessions/` is per-session capture — gitignored by default.
- `.stoa/` is derived state — safe to delete; rebuilt by `stoa rebuild`.
