//! Schema validation for parsed wiki frontmatter.
//!
//! `validate_page` is the spine of `stoa schema --check`. It takes the
//! raw YAML text (so we can produce useful errors for pages that fail to
//! parse, e.g. unknown `kind:` or missing `title:`) plus the path-derived
//! page id, and returns a list of [`ValidationError`].

use serde_yaml::Value;

use crate::error::ValidationError;
use crate::frontmatter::{Frontmatter, KindData};
use crate::schema::Schema;

/// Validate a page's raw frontmatter YAML against `schema`.
///
/// The `path_id` is the id derived from the file path (e.g. `ent-redis`
/// from `wiki/entities/ent-redis.md`). It's used as the page identifier
/// in error messages so the CLI can still pin-point the offending file
/// even when the YAML fails to parse.
#[must_use]
pub fn validate_page(yaml: &str, path_id: &str, schema: &Schema) -> Vec<ValidationError> {
    match serde_yaml::from_str::<Frontmatter>(yaml) {
        Ok(fm) => validate_parsed(&fm, path_id, schema),
        Err(err) => vec![classify_parse_error(yaml, path_id, &err)],
    }
}

fn validate_parsed(fm: &Frontmatter, path_id: &str, schema: &Schema) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let pid = if fm.id.is_empty() { path_id } else { &fm.id };
    match &fm.kind_data {
        KindData::Entity(data) => {
            if !schema.allows_entity_type(data.entity_type.as_str()) {
                errors.push(ValidationError::new(
                    pid,
                    "type",
                    format!(
                        "unknown entity type `{}` (allowed: {})",
                        data.entity_type.as_str(),
                        schema.entity_types().join(", "),
                    ),
                ));
            }
            push_relationship_errors(&data.relationships, pid, schema, &mut errors);
        },
        KindData::Concept(data) => {
            push_relationship_errors(&data.relationships, pid, schema, &mut errors);
        },
        KindData::Synthesis(_) => {},
    }
    errors
}

fn push_relationship_errors(
    rels: &[crate::relationship::Relationship],
    pid: &str,
    schema: &Schema,
    out: &mut Vec<ValidationError>,
) {
    for rel in rels {
        if !schema.allows_relationship_type(rel.kind.as_str()) {
            out.push(ValidationError::new(
                pid,
                "relationship.type",
                format!(
                    "unknown relationship type `{}` (allowed: {})",
                    rel.kind.as_str(),
                    schema.relationship_types().join(", "),
                ),
            ));
        }
    }
}

fn classify_parse_error(yaml: &str, path_id: &str, err: &serde_yaml::Error) -> ValidationError {
    let pid = extract_id(yaml).unwrap_or_else(|| path_id.to_owned());
    let msg = err.to_string();
    let field = guess_field(&msg);
    ValidationError::new(pid, field, msg)
}

fn extract_id(yaml: &str) -> Option<String> {
    let value: Value = serde_yaml::from_str(yaml).ok()?;
    value.get("id")?.as_str().map(str::to_owned)
}

fn guess_field(msg: &str) -> String {
    // serde_yaml messages we care about:
    //   "unknown variant `nonsense`, expected one of `entity`, ..."
    //   "missing field `title`"
    //   "type `not-a-real-type` ..." (rare, our schema validation catches these)
    let lower = msg.to_ascii_lowercase();
    if lower.contains("missing field `title`") || lower.contains("`title`") {
        "title".to_owned()
    } else if lower.contains("missing field `kind`") || lower.contains("unknown variant") {
        "kind".to_owned()
    } else if lower.contains("missing field `status`") || lower.contains("`status`") {
        "status".to_owned()
    } else if lower.contains("missing field `type`") || lower.contains("`type`") {
        "type".to_owned()
    } else if lower.contains("missing field `created`") || lower.contains("`created`") {
        "created".to_owned()
    } else if lower.contains("missing field `updated`") || lower.contains("`updated`") {
        "updated".to_owned()
    } else {
        "frontmatter".to_owned()
    }
}
