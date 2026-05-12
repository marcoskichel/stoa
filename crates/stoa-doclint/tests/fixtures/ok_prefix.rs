//! Fixture: bare `//` comments justified by an allowed intent prefix. Must
//! NOT be flagged.

pub fn x() {
    // SAFETY: caller holds the write lock; see WAL invariant.
    // PERF: branchless path measured 1.4x faster on N<=200 (bench-2026-05-08).
    // NOTE: ordering matters here — flush must precede commit on failure paths.
    // FIXME: rusqlite 0.38 panics on empty BLOB; remove pad when upstream fixes.
    // HACK: workaround for `cross-rs` wine seccomp on Linux 7.x.
    // WHY: keep the loop unrolled — codegen regresses on the loop form.
}
