//! Property-based round-trip test for the public `Frontmatter` API.
//!
//! Asserts the invariant: for any frontmatter that parses successfully, the
//! sequence `parse → serialize → parse` is the identity. This protects against
//! lossy or order-dependent serde derives in `stoa_core::Frontmatter`.
//!
//! Spec source: [ARCHITECTURE.md §2 Wiki data model — Frontmatter schema].

use proptest::prelude::*;
use stoa_core::Frontmatter;

fn kind_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("entity"), Just("concept"), Just("synthesis")]
}

fn status_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("active"),
        Just("superseded"),
        Just("stale"),
        Just("deprecated")
    ]
}

fn entity_type_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("person"),
        Just("project"),
        Just("library"),
        Just("service"),
        Just("tool"),
        Just("file"),
        Just("decision"),
    ]
}

fn build_yaml(id: &str, kind: &str, title: &str, status: &str, entity_type: &str) -> String {
    let extra = if kind == "entity" {
        format!("type: {entity_type}\n")
    } else {
        String::new()
    };
    format!(
        "id: {id}\n\
         kind: {kind}\n\
         title: \"{title}\"\n\
         status: {status}\n\
         created: 2026-05-12T00:00:00Z\n\
         updated: 2026-05-12T00:00:00Z\n\
         {extra}",
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn frontmatter_roundtrips(
        id in "[a-z]{2,4}-[a-z][a-z0-9-]{0,20}",
        kind in kind_strategy(),
        title in "[A-Za-z][A-Za-z0-9 ]{1,20}",
        status in status_strategy(),
        entity_type in entity_type_strategy(),
    ) {
        let yaml = build_yaml(&id, kind, &title, status, entity_type);
        let parsed: Frontmatter = serde_yaml::from_str(&yaml)
            .map_err(|e| TestCaseError::fail(format!("parse failed: {e}\nyaml:\n{yaml}")))?;
        let reserialized = serde_yaml::to_string(&parsed)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let reparsed: Frontmatter = serde_yaml::from_str(&reserialized)
            .map_err(|e| TestCaseError::fail(format!("reparse failed: {e}\nyaml:\n{reserialized}")))?;
        prop_assert_eq!(parsed, reparsed);
    }
}

#[test]
fn unknown_kind_is_rejected() {
    let yaml = "id: ent-bad\nkind: nonsense\ntitle: Bad\nstatus: active\n\
                created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z\n";
    let parsed: Result<Frontmatter, _> = serde_yaml::from_str(yaml);
    assert!(parsed.is_err(), "unknown kind must not parse");
}

#[test]
fn unknown_status_is_rejected() {
    let yaml = "id: ent-bad\nkind: entity\ntitle: Bad\ntype: library\nstatus: limbo\n\
                created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z\n";
    let parsed: Result<Frontmatter, _> = serde_yaml::from_str(yaml);
    assert!(parsed.is_err(), "unknown status must not parse");
}

#[test]
fn entity_requires_type_field() {
    let yaml = "id: ent-bad\nkind: entity\ntitle: Bad\nstatus: active\n\
                created: 2026-05-12T00:00:00Z\nupdated: 2026-05-12T00:00:00Z\n";
    // NOTE: intentionally missing `type:` — entities require it per ARCHITECTURE §2.
    let parsed: Result<Frontmatter, _> = serde_yaml::from_str(yaml);
    assert!(parsed.is_err(), "entity without `type:` must not parse");
}
