//! E2E quality gate: `stoa init` — workspace scaffolding.
//!
//! Spec source: [ROADMAP.md M2] + [ARCHITECTURE.md §1 Layout on disk].

mod common;

use common::{assert_paths_exist, contains, exists, init, read_file, stoa, workspace};

const EXPECTED_SCAFFOLD: &[&str] = &[
    "STOA.md",
    ".gitignore",
    "raw",
    "wiki",
    "wiki/index.md",
    "wiki/log.md",
    "wiki/entities",
    "wiki/concepts",
    "wiki/synthesis",
    "sessions",
    ".stoa",
];

#[test]
fn init_succeeds_in_empty_dir() {
    let ws = workspace();
    let out = stoa(&ws, &["init"]);
    assert!(out.status.success(), "init must succeed on empty dir");
}

#[test]
fn init_scaffolds_every_expected_path() {
    let ws = workspace();
    init(&ws);
    assert_paths_exist(&ws, EXPECTED_SCAFFOLD);
}

#[test]
fn init_is_idempotent() {
    let ws = workspace();
    init(&ws);
    let probe_rel = "wiki/entities/probe.md";
    common::write_file(&ws, probe_rel, "user content\n");
    let second = stoa(&ws, &["init"]);
    assert!(
        second.status.success(),
        "second init must not error: {}",
        common::stderr(&second),
    );
    assert!(exists(&ws, probe_rel), "idempotent init must not delete user content");
}

#[test]
fn init_writes_default_stoa_md() {
    let ws = workspace();
    init(&ws);
    let body = read_file(&ws, "STOA.md");
    assert!(!body.trim().is_empty(), "STOA.md must not be empty");
    // NOTE: STOA.md default template must surface vocabulary per ARCHITECTURE §3.
    assert!(
        contains(&body, "library") || contains(&body, "decision"),
        "default STOA.md missing entity type vocabulary: {body}",
    );
    assert!(
        contains(&body, "depends_on") || contains(&body, "supersedes"),
        "default STOA.md missing relationship vocabulary: {body}",
    );
}

#[test]
fn init_writes_gitignore_for_derived_state() {
    let ws = workspace();
    init(&ws);
    let body = read_file(&ws, ".gitignore");
    assert!(
        contains(&body, ".stoa") && contains(&body, "sessions"),
        ".gitignore must cover .stoa/ and sessions/ per ARCHITECTURE §1: {body}",
    );
}

#[test]
fn init_index_md_is_present_but_may_be_empty() {
    let ws = workspace();
    init(&ws);
    assert!(exists(&ws, "wiki/index.md"), "wiki/index.md must exist post-init");
}

#[test]
fn init_log_md_records_init_event() {
    let ws = workspace();
    init(&ws);
    let log = read_file(&ws, "wiki/log.md");
    // NOTE: log.md is an append-only journal per ARCHITECTURE §2 — init is an event.
    assert!(!log.trim().is_empty(), "log.md must record init event, not be empty");
    assert!(contains(&log, "init"), "log.md must mention `init`; got: {log:?}");
}

#[test]
fn init_in_corrupt_partial_workspace_repairs_missing_dirs() {
    let ws = workspace();
    // NOTE: deleting `.stoa/` is the documented disaster-recovery path (ARCH §1).
    init(&ws);
    std::fs::remove_dir_all(ws.path().join(".stoa")).unwrap();
    let out = stoa(&ws, &["init"]);
    assert!(out.status.success(), "init must repair missing .stoa/");
    assert!(exists(&ws, ".stoa"), ".stoa/ must be re-created");
}
