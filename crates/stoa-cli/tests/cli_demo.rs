//! E2E quality gate: full M2 demo round-trip.
//!
//! Reproduces the ROADMAP M2 demo verbatim:
//!
//! > "User runs `stoa init`, creates 3 entity pages with `stoa write`,
//! > runs `stoa schema --check`, sees validation pass; introduces a
//! > deliberate frontmatter error, sees `stoa schema --check` fail with
//! > a useful message."
//!
//! This test is the single load-bearing exit-criteria check.

mod common;

use assert_fs::TempDir;

use common::{contains, init, stderr, stoa, workspace, write_file};

const ENTITIES: &[(&str, &str, &str, &str)] = &[
    ("ent-redis", "Redis", "library", "An in-memory key-value store."),
    ("ent-postgres", "Postgres", "library", "Relational database."),
    ("ent-claude-code", "Claude Code", "tool", "Anthropic's CLI agent."),
];

fn write_one(ws: &TempDir, id: &str, title: &str, entity_type: &str, body: &str) {
    let fm = format!("kind: entity\ntitle: {title}\ntype: {entity_type}\nstatus: active\n");
    write_file(ws, "tmp/fm.yaml", &fm);
    write_file(ws, "tmp/body.md", &format!("{body}\n"));
    let out = stoa(
        ws,
        &[
            "write",
            id,
            "--frontmatter",
            "tmp/fm.yaml",
            "--body",
            "tmp/body.md",
        ],
    );
    assert!(out.status.success(), "write `{id}` failed: {}", stderr(&out));
}

fn assert_schema_passes(ws: &TempDir) {
    let check = stoa(ws, &["schema", "--check"]);
    assert!(
        check.status.success(),
        "valid workspace must pass schema check: {}",
        stderr(&check),
    );
}

fn assert_read_round_trip(ws: &TempDir) {
    for &(id, title, _et, body) in ENTITIES {
        let out = stoa(ws, &["read", id]);
        assert!(out.status.success(), "read `{id}` failed");
        let printed = common::stdout(&out);
        assert!(contains(&printed, title), "read `{id}` missing title");
        assert!(contains(&printed, body), "read `{id}` missing body");
    }
}

fn break_one_page(ws: &TempDir) {
    let bad = "---\nid: ent-broken\nkind: entity\ntitle: Broken\n\
               type: not-a-real-type\nstatus: active\n\
               created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z\n---\n\
               broken body\n";
    write_file(ws, "wiki/entities/ent-broken.md", bad);
}

fn assert_schema_rejects_broken(ws: &TempDir) {
    let fail = stoa(ws, &["schema", "--check"]);
    assert!(!fail.status.success(), "schema check must reject `not-a-real-type`");
    let err = stderr(&fail);
    assert!(
        contains(&err, "ent-broken") || contains(&err, "not-a-real-type"),
        "error must point to the broken page or its bad value: {err}",
    );
}

#[test]
fn full_m2_demo_round_trip() {
    let ws = workspace();
    init(&ws);
    for &(id, title, et, body) in ENTITIES {
        write_one(&ws, id, title, et, body);
    }
    assert_schema_passes(&ws);
    assert_read_round_trip(&ws);
    break_one_page(&ws);
    assert_schema_rejects_broken(&ws);
}
