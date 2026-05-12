//! E2E quality gate: `stoa schema` + `stoa schema --check`.
//!
//! Validates frontmatter against the `STOA.md` vocabulary. Failures must be
//! actionable: the error must name the offending page id and the rule violated.
//!
//! Spec source: [ROADMAP.md M2] + [ARCHITECTURE.md §3 Schema].

mod common;

use assert_fs::TempDir;

use common::{contains, init, stderr, stoa, workspace, write_file};

fn write_page(ws: &TempDir, dir: &str, id: &str, frontmatter: &str) {
    let body = "page body\n";
    let page = format!("---\nid: {id}\n{frontmatter}\n---\n{body}");
    write_file(ws, &format!("wiki/{dir}/{id}.md"), &page);
}

#[test]
fn schema_prints_stoa_md_contents() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["schema"]);
    assert!(out.status.success(), "schema must succeed: {}", stderr(&out));
    let text = common::stdout(&out);
    assert!(!text.trim().is_empty(), "schema must print STOA.md contents");
}

#[test]
fn schema_check_passes_on_fresh_workspace() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(out.status.success(), "fresh workspace must pass schema check: {}", stderr(&out));
}

#[test]
fn schema_check_passes_with_valid_entity_page() {
    let ws = workspace();
    init(&ws);
    write_page(
        &ws,
        "entities",
        "ent-redis",
        "kind: entity\ntitle: Redis\ntype: library\nstatus: active\n\
         created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z",
    );
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(out.status.success(), "valid page must pass check: {}", stderr(&out));
}

#[test]
fn schema_check_rejects_unknown_kind() {
    let ws = workspace();
    init(&ws);
    write_page(
        &ws,
        "entities",
        "ent-bad",
        "kind: nonsense\ntitle: bad\nstatus: active\n\
         created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z",
    );
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "unknown kind must fail check");
    let err = stderr(&out);
    assert!(
        contains(&err, "kind") && contains(&err, "ent-bad"),
        "error must name the field and page: {err}",
    );
}

#[test]
fn schema_check_rejects_unknown_entity_type() {
    let ws = workspace();
    init(&ws);
    write_page(
        &ws,
        "entities",
        "ent-bad",
        "kind: entity\ntitle: Bad\ntype: not-a-real-type\nstatus: active\n\
         created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z",
    );
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "unknown entity.type must fail");
    let err = stderr(&out);
    assert!(
        contains(&err, "not-a-real-type") || contains(&err, "type"),
        "error must name the offending entity type: {err}",
    );
}

#[test]
fn schema_check_rejects_missing_required_field() {
    let ws = workspace();
    init(&ws);
    write_page(
        &ws,
        "entities",
        "ent-missing-title",
        // No `title:` field.
        "kind: entity\ntype: library\nstatus: active\n\
         created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z",
    );
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "missing required field must fail");
    let err = stderr(&out);
    assert!(
        contains(&err, "title") || contains(&err, "required"),
        "error must mention missing field: {err}",
    );
}

#[test]
fn schema_check_rejects_invalid_relationship_type() {
    let ws = workspace();
    init(&ws);
    let fm = "kind: entity\ntitle: Bad\ntype: library\nstatus: active\n\
              created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z\n\
              relationships:\n  - { type: hates, target: ent-other }";
    write_page(&ws, "entities", "ent-bad-rel", fm);
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "invalid relationship type must fail");
    let err = stderr(&out);
    assert!(
        contains(&err, "hates") || contains(&err, "relationship"),
        "error must mention bad relationship type: {err}",
    );
}

#[test]
fn schema_check_rejects_invalid_status() {
    let ws = workspace();
    init(&ws);
    write_page(
        &ws,
        "entities",
        "ent-bad-status",
        "kind: entity\ntitle: Bad\ntype: library\nstatus: limbo\n\
         created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z",
    );
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "invalid status must fail");
    let err = stderr(&out);
    assert!(
        contains(&err, "status") || contains(&err, "limbo"),
        "error must name the bad status value: {err}",
    );
}

#[test]
fn schema_check_outside_workspace_exits_non_zero() {
    let ws = workspace();
    // Deliberately no init.
    let out = stoa(&ws, &["schema", "--check"]);
    assert!(!out.status.success(), "schema check must fail outside workspace");
}
