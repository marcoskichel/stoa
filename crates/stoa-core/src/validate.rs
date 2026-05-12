//! Schema validation for parsed wiki frontmatter.
//!
//! `validate_page` is the spine of `stoa schema --check`. It runs in two
//! passes — first inspect the raw YAML mapping for missing required fields
//! and invalid enum values (so error attribution stays structural, not
//! text-matched), then attempt the typed [`Frontmatter`] parse for
//! relationship-level checks.

use serde_yaml::{Mapping, Value};

use crate::error::ValidationError;
use crate::frontmatter::{Frontmatter, KindData};
use crate::schema::Schema;

const REQUIRED_FIELDS: &[&str] = &["id", "kind", "title", "status", "created", "updated"];

/// Validate a page's raw frontmatter YAML against `schema`.
///
/// The `path_id` is the id derived from the file path (e.g. `ent-redis`
/// from `wiki/entities/ent-redis.md`). It's used as the page identifier
/// in error messages so the CLI can pin-point the offending file even
/// when the YAML is structurally broken.
#[must_use]
pub fn validate_page(yaml: &str, path_id: &str, schema: &Schema) -> Vec<ValidationError> {
    let map = match parse_mapping(yaml, path_id) {
        Ok(m) => m,
        Err(err) => return vec![err],
    };
    let pid = map_str(&map, "id").map_or_else(|| path_id.to_owned(), str::to_owned);
    let mut errors = check_required(&map, &pid);
    errors.extend(check_enums(&map, &pid, schema));
    if errors.is_empty() {
        errors.extend(check_typed(yaml, &pid, schema));
    }
    errors
}

fn parse_mapping(yaml: &str, path_id: &str) -> Result<Mapping, ValidationError> {
    let value: Value = serde_yaml::from_str(yaml)
        .map_err(|e| ValidationError::new(path_id, "frontmatter", format!("yaml syntax: {e}")))?;
    match value {
        Value::Mapping(m) => Ok(m),
        _ => Err(ValidationError::new(path_id, "frontmatter", "not a YAML mapping")),
    }
}

fn check_required(map: &Mapping, pid: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for field in REQUIRED_FIELDS {
        if !map_has(map, field) {
            let msg = format!("missing required field `{field}`");
            errors.push(ValidationError::new(pid, *field, msg));
        }
    }
    if matches!(map_str(map, "kind"), Some("entity")) && !map_has(map, "type") {
        let msg = "missing required field `type` for entity".to_owned();
        errors.push(ValidationError::new(pid, "type", msg));
    }
    errors
}

fn check_enums(map: &Mapping, pid: &str, schema: &Schema) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let Some(k) = map_str(map, "kind").filter(|v| !schema.allows_kind(v)) {
        let msg = format!("unknown kind `{k}` (allowed: entity, concept, synthesis)");
        errors.push(ValidationError::new(pid, "kind", msg));
    }
    if let Some(s) = map_str(map, "status").filter(|v| !schema.allows_status(v)) {
        let allowed = "active, superseded, stale, deprecated";
        let msg = format!("unknown status `{s}` (allowed: {allowed})");
        errors.push(ValidationError::new(pid, "status", msg));
    }
    if matches!(map_str(map, "kind"), Some("entity"))
        && let Some(t) = map_str(map, "type").filter(|v| !schema.allows_entity_type(v))
    {
        let allowed = schema.entity_types().join(", ");
        let msg = format!("unknown entity type `{t}` (allowed: {allowed})");
        errors.push(ValidationError::new(pid, "type", msg));
    }
    errors
}

fn check_typed(yaml: &str, pid: &str, schema: &Schema) -> Vec<ValidationError> {
    match serde_yaml::from_str::<Frontmatter>(yaml) {
        Ok(fm) => relationship_errors(&fm, pid, schema),
        Err(err) => vec![ValidationError::new(pid, "frontmatter", err.to_string())],
    }
}

fn relationship_errors(fm: &Frontmatter, pid: &str, schema: &Schema) -> Vec<ValidationError> {
    let rels: &[crate::relationship::Relationship] = match &fm.kind_data {
        KindData::Entity(d) => &d.relationships,
        KindData::Concept(d) => &d.relationships,
        KindData::Synthesis(_) => return Vec::new(),
    };
    let mut errors = Vec::new();
    for rel in rels {
        if !schema.allows_relationship_type(rel.kind.as_str()) {
            let allowed = schema.relationship_types().join(", ");
            let msg = format!("unknown relationship type `{}` (allowed: {allowed})", rel.kind.as_str());
            errors.push(ValidationError::new(pid, "relationship.type", msg));
        }
    }
    errors
}

fn map_has(map: &Mapping, key: &str) -> bool {
    map.get(Value::String(key.to_owned())).is_some()
}

fn map_str<'a>(map: &'a Mapping, key: &str) -> Option<&'a str> {
    map.get(Value::String(key.to_owned()))?.as_str()
}
