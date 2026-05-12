//! Typed relationships + entity-type vocabulary (ARCHITECTURE §5).

use serde::{Deserialize, Serialize};

/// Entity type defaults (ARCHITECTURE §5 — "Entity types"). The schema
/// (`STOA.md`) may extend this vocabulary per workspace.
pub const DEFAULT_ENTITY_TYPES: &[&str] = &[
    "person", "project", "library", "service", "tool", "file", "decision", "concept",
];

/// Relationship type defaults (ARCHITECTURE §5 — "Relationship types").
pub const DEFAULT_RELATIONSHIP_TYPES: &[&str] = &[
    "uses",
    "depends_on",
    "instance_of",
    "caused",
    "fixed",
    "supersedes",
    "contradicts",
    "cites",
    "mentions",
];

/// Wrapper around an entity-type string. Strings are kept open so workspaces
/// can extend the vocabulary in `STOA.md`; validation against the active
/// schema is the caller's job (see `validate::validate_page`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityType(pub String);

impl EntityType {
    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Wrapper around a relationship-type string. Stored as a plain string for
/// the same reason as [`EntityType`] — schemas can extend the vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelationshipType(pub String);

impl RelationshipType {
    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A typed relationship between two pages (ARCHITECTURE §2 + §5).
///
/// Confidence + source provenance are optional — they're populated by the
/// LLM/extractor and absent on hand-authored pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship kind, validated against the schema vocabulary.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
    /// Page id (`ent-*`, `con-*`, `syn-*`) this relationship points at.
    pub target: String,
    /// Optional 0.0..=1.0 numeric confidence (ARCHITECTURE §4.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Optional supporting source paths (typically under `raw/`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
}
