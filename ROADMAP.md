# Roadmap

The pivot on 2026-05-13 collapsed the previous M0..M5 milestones — those built a from-scratch retrieval substrate that MemPalace already ships better. What remains is the shorter list of work to reach v0.1.

The implementation rationale lives in [ARCHITECTURE.md](./ARCHITECTURE.md); the pivot decision lives in [docs/adr/0001-mempalace-pivot.md](./docs/adr/0001-mempalace-pivot.md).

Sizes are t-shirt: **S** = days, **M** = 1–2 weeks, **L** = 3–4 weeks.

---

## M-Pivot — the cut (shipping now)

**Size**: M (this PR)

**Deliverable**:

- Deletes the from-scratch retrieval stack (`stoa-queue`, `stoa-capture`, `stoa-bench`, `stoa-recall/backends/local-chroma-sqlite`, all viz/render crates; Python `stoa-shared`, `stoa-embed`, `stoa-recall` sidecar, `stoa-bench-judge`).
- Adds `stoa-recalld` (Python daemon hosting MemPalace, Unix socket, 5 RPCs).
- Rewires `stoa-hook` to send `mine` RPCs.
- Rewires `stoa-inject-hook` to handle BOTH `SessionStart` and `UserPromptSubmit`, fetch wiki hits via the daemon's `search` RPC, wrap in the MINJA envelope.
- Rewires `stoa` CLI around the new shape (`init`, `daemon`, `hook`, `schema`, `write`, `read`, `query`, `inject log`).
- Rewires `stoa-harvest` and `stoa-crystallize` to drive the daemon over the socket.

**Exit criteria**:

- `cargo test --workspace` is green.
- `uv run pytest -q` is green (unit-level, no daemon).
- A documented manual smoke test passes: `stoa init`, `stoa daemon start`, `stoa write` a page, `stoa query` returns it, `stoa-inject-hook` produces a non-empty `additionalContext` when fed a fixture payload.
- `cargo install --path crates/stoa-cli --locked` succeeds.

---

## M-v0.1 — release on-ramp

**Size**: S

**Deliverable**:

- README + docs site (mkdocs) rewritten around the pivot.
- `release-plz.toml` updated for the surviving crates only (`stoa-core`, `stoa-recall`, `stoa-cli`, `stoa-hooks`, `stoa-inject-hooks`, `stoa-doclint`).
- Crates.io 0.1.0 republish (or yank + bump to 0.1.1) for the surviving crates; obsolete crates yanked.
- Python sidecar (`stoa-recalld`, `stoa-harvest`, `stoa-crystallize`) published to PyPI.
- `cargo install stoa-cli` works on fresh macOS + Linux machines (`stoa daemon start` requires `mempalace` already installed via uv/pip).
- Tagged `v0.1.0` push triggers `release.yml` cross-compile across 5 targets.

**Exit criteria**:

- Public release notes published.
- The README install path works end-to-end on a fresh VM.

---

## M-v0.1.x — wiki tooling

**Size**: M

**Deliverable**:

- Bidirectional drawer↔wiki links exposed in `stoa read` output ("Sources: drawer-id-1, drawer-id-2").
- `stoa write --interactive` — guided page creation from a recent drawer.
- `stoa harvest run --since <ts>` — incremental cursor so harvest catches up rather than reading everything.
- `stoa crystallize --weekly` — scheduled synthesis trigger.
- `stoa lint` — orphan page detection, broken-relationship detection, stale-frontmatter timestamp warnings.
- `stoa index rebuild` — re-mirror `wiki/*.md` into MemPalace without using the LLM (for hand-edited pages).

**Exit criteria**:

- A user editing the wiki by hand can rebuild the index without restarting the daemon.

---

## M-v0.2 — multi-platform

**Size**: L

**Deliverable**:

- Cursor + Codex hook adapters (`stoa hook install --platform cursor|codex`).
- Windows-native daemon transport (named pipe or TCP fallback for Unix-socket).
- All-Rust harvest worker as an experimental opt-in (`STOA_HARVEST_BACKEND=rust`).
- Multi-workspace daemon (one `stoa-recalld` serving N workspaces).

**Exit criteria**:

- Cursor + Codex docs published; Windows install path documented.

---

## M-v0.3 — paid layer scaffolding

**Size**: L

**Deliverable**:

- Sync protocol design (E2E-encrypted, server-cycle the index, never the markdown).
- Team mode design (CRDT-friendly wiki edits, attribution, conflict resolution).
- Hosted instance prototype behind a closed beta.
- The OSS core stays exactly as shipped — no required cloud, no required account.

---

## Out of scope for v0.1

- Visualization (the old `stoa-viz` + render crates are gone).
- LongMemEval / MemBench benchmark publication (MemPalace already publishes; Stoa cites).
- MCP server wrapper (MemPalace ships a 29-tool MCP server already; we recommend it for agents that prefer tool-calls over hooks).
- An always-on harvest watcher (one-shot `stoa-harvest run` is the v0.1 contract).
