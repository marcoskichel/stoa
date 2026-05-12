//! E2E quality gate: `stoa write` + `stoa read` round-trip.
//!
//! Spec source: [ROADMAP.md M2] + [ARCHITECTURE.md §2 Wiki data model].

mod common;

use assert_fs::TempDir;
use assert_fs::prelude::PathChild;

use common::{contains, init, read_file, stderr, stoa, workspace, write_file};

const ENTITY_FM: &str = "\
kind: entity
title: Redis
type: library
status: active
";

const ENTITY_BODY: &str = "Redis is an in-memory key-value store.\n";

fn write_entity(ws: &TempDir, id: &str) -> std::process::Output {
    write_file(ws, "tmp/fm.yaml", ENTITY_FM);
    write_file(ws, "tmp/body.md", ENTITY_BODY);
    stoa(
        ws,
        &[
            "write",
            id,
            "--frontmatter",
            "tmp/fm.yaml",
            "--body",
            "tmp/body.md",
        ],
    )
}

#[test]
fn write_entity_creates_page_at_expected_path() {
    let ws = workspace();
    init(&ws);
    let out = write_entity(&ws, "ent-redis");
    assert!(out.status.success(), "write must succeed: {}", stderr(&out));
    assert!(
        ws.child("wiki/entities/ent-redis.md").path().exists(),
        "page must land at wiki/entities/<id>.md",
    );
}

#[test]
fn write_then_read_roundtrips_body() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let out = stoa(&ws, &["read", "ent-redis"]);
    assert!(out.status.success(), "read must succeed");
    let printed = common::stdout(&out);
    assert!(
        contains(&printed, "in-memory key-value store"),
        "read must print body verbatim; got: {printed:?}",
    );
    assert!(
        contains(&printed, "Redis"),
        "read must include frontmatter title; got: {printed:?}",
    );
}

#[test]
fn write_persists_required_frontmatter_fields() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let page = read_file(&ws, "wiki/entities/ent-redis.md");
    for required in ["id:", "kind:", "title:", "created:", "updated:", "status:"] {
        assert!(
            contains(&page, required),
            "missing required frontmatter field `{required}`: {page}",
        );
    }
}

#[test]
fn write_assigns_id_from_cli_argument() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let page = read_file(&ws, "wiki/entities/ent-redis.md");
    assert!(
        contains(&page, "id: ent-redis"),
        "frontmatter id must match CLI arg; got: {page}",
    );
}

/// Extract the `updated:` field from a page's frontmatter as a raw string.
/// RFC-3339 fixed-width `Z` timestamps sort correctly lexicographically.
#[expect(
    clippy::expect_used,
    reason = "Test helper; structural failure (no frontmatter, no `updated:`) is a test bug."
)]
fn extract_updated(page: &str) -> String {
    let after_open = page
        .strip_prefix("---\n")
        .expect("page starts with frontmatter open");
    let end = after_open
        .find("\n---")
        .expect("page has frontmatter close");
    let yaml = &after_open[..end];
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).expect("frontmatter is valid YAML");
    value
        .get("updated")
        .and_then(|v| v.as_str())
        .expect("frontmatter has an `updated:` field")
        .to_owned()
}

#[test]
fn write_updates_existing_page_bumps_updated_timestamp() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let first = read_file(&ws, "wiki/entities/ent-redis.md");
    let first_updated = extract_updated(&first);
    write_file(&ws, "tmp/body.md", "Edited body.\n");
    let out = stoa(&ws, &["write", "ent-redis", "--body", "tmp/body.md"]);
    assert!(out.status.success(), "rewrite must succeed: {}", stderr(&out));
    let second = read_file(&ws, "wiki/entities/ent-redis.md");
    let second_updated = extract_updated(&second);
    assert!(
        second_updated >= first_updated,
        "`updated` must not regress (first: {first_updated}, second: {second_updated})",
    );
    assert!(contains(&second, "Edited body."));
}

#[test]
fn write_appends_event_to_log_md() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let log = read_file(&ws, "wiki/log.md");
    assert!(contains(&log, "ent-redis"), "log.md must record the new page id: {log}");
}

#[test]
fn write_updates_index_md() {
    let ws = workspace();
    init(&ws);
    let _out = write_entity(&ws, "ent-redis");
    let index = read_file(&ws, "wiki/index.md");
    assert!(
        contains(&index, "ent-redis") || contains(&index, "Redis"),
        "index.md must catalog the new page: {index}",
    );
}

#[test]
fn read_missing_id_exits_non_zero_with_useful_error() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["read", "ent-does-not-exist"]);
    assert!(!out.status.success(), "missing id must exit non-zero");
    let err = stderr(&out);
    assert!(
        contains(&err, "ent-does-not-exist") || contains(&err, "not found"),
        "error must mention the missing id or `not found`: {err}",
    );
}

#[test]
fn write_concept_page_lands_in_concepts_directory() {
    let ws = workspace();
    init(&ws);
    let fm = "kind: concept\ntitle: Rate limiting\nstatus: active\n";
    write_file(&ws, "tmp/fm.yaml", fm);
    write_file(&ws, "tmp/body.md", "A concept page.\n");
    let out = stoa(
        &ws,
        &[
            "write",
            "con-rate-limiting",
            "--frontmatter",
            "tmp/fm.yaml",
            "--body",
            "tmp/body.md",
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        ws.child("wiki/concepts/con-rate-limiting.md")
            .path()
            .exists()
    );
}

#[test]
fn write_synthesis_page_lands_in_synthesis_directory() {
    let ws = workspace();
    init(&ws);
    let fm = "kind: synthesis\ntitle: Redis vs Memcached\nstatus: active\n\
              question: \"Which session store?\"\ninputs: [ent-redis]\n";
    write_file(&ws, "tmp/fm.yaml", fm);
    write_file(&ws, "tmp/body.md", "Synthesis body.\n");
    let out = stoa(
        &ws,
        &[
            "write",
            "syn-redis-vs-memcached",
            "--frontmatter",
            "tmp/fm.yaml",
            "--body",
            "tmp/body.md",
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        ws.child("wiki/synthesis/syn-redis-vs-memcached.md")
            .path()
            .exists()
    );
}

#[test]
fn write_without_init_exits_non_zero() {
    let ws = workspace();
    // No init: workspace not scaffolded.
    write_file(&ws, "tmp/fm.yaml", ENTITY_FM);
    write_file(&ws, "tmp/body.md", ENTITY_BODY);
    let out = stoa(
        &ws,
        &[
            "write",
            "ent-redis",
            "--frontmatter",
            "tmp/fm.yaml",
            "--body",
            "tmp/body.md",
        ],
    );
    assert!(!out.status.success(), "write must fail outside a stoa workspace");
}
