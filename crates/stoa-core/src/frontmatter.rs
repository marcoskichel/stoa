//! YAML frontmatter — the in-memory shape of a wiki page's metadata block.
//!
//! Required fields (per ARCHITECTURE §2):
//!
//! ```yaml
//! id: ent-redis
//! kind: entity            # entity | concept | synthesis
//! title: Redis
//! created: 2026-05-12T14:32:00Z
//! updated: 2026-05-12T18:01:00Z
//! status: active          # active | superseded | stale | deprecated
//! ```
//!
//! Kind-specific extras land in the [`KindData`] variant. Entities **must**
//! supply `type:`; concepts and synthesis pages have optional extras.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kind::{Kind, Status};
use crate::relationship::{EntityType, Relationship};

/// Parsed frontmatter block. Round-trips through `serde_yaml` (asserted by
/// the proptest `frontmatter_roundtrips` in `tests/frontmatter_roundtrip.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Stable page identifier (e.g. `ent-redis`, `con-rag`, `syn-…`).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Lifecycle status.
    pub status: Status,
    /// RFC-3339 / ISO-8601 creation timestamp.
    pub created: DateTime<Utc>,
    /// RFC-3339 / ISO-8601 last-update timestamp.
    pub updated: DateTime<Utc>,

    /// Kind-discriminated extras (the `kind:` field plus per-kind data).
    ///
    /// Flattened so the YAML stays flat — `kind: entity` sits alongside
    /// `id:` and `title:`, not nested.
    #[serde(flatten)]
    pub kind_data: KindData,
}

/// Kind-specific frontmatter fields. The `kind:` discriminator lives on this
/// enum so an entity is forced to carry `type:` and a synthesis is forced
/// to carry `inputs:` / `question:` (when present).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum KindData {
    /// `kind: entity` — required `type:`, optional aliases + relationships.
    Entity(EntityData),
    /// `kind: concept` — optional relationships.
    Concept(ConceptData),
    /// `kind: synthesis` — optional `inputs:` / `question:`.
    Synthesis(SynthesisData),
}

/// Entity-specific frontmatter (ARCHITECTURE §2 — entity block).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityData {
    /// Entity sub-type — validated against the schema vocabulary.
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    /// Optional alternate names / abbreviations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Optional typed relationships out from this entity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<Relationship>,
}

/// Concept-specific frontmatter (ARCHITECTURE §2 — concept block).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptData {
    /// Optional typed relationships.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<Relationship>,
}

/// Synthesis-specific frontmatter (ARCHITECTURE §2 — synthesis block).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisData {
    /// Source pages + raw artifacts that fed this synthesis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// The question the synthesis answers, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

impl Frontmatter {
    /// Convenience accessor for `kind:` as the canonical lowercase string.
    #[must_use]
    pub fn kind(&self) -> Kind {
        match &self.kind_data {
            KindData::Entity(_) => Kind::Entity,
            KindData::Concept(_) => Kind::Concept,
            KindData::Synthesis(_) => Kind::Synthesis,
        }
    }
}
