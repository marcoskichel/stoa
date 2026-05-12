//! Enums for page `kind` and `status` (ARCHITECTURE §2).

use serde::{Deserialize, Serialize};

/// Wiki page kind (ARCHITECTURE §2). Mirrors the directory layout under
/// `wiki/{entities,concepts,synthesis}/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A real thing with identity over time (person, project, library, …).
    Entity,
    /// An abstract topic or pattern.
    Concept,
    /// A cross-cutting essay built from entities, concepts, and raw sources.
    Synthesis,
}

impl Kind {
    /// Canonical lowercase string for the kind (matches the YAML form).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Concept => "concept",
            Self::Synthesis => "synthesis",
        }
    }

    /// Default allow-list — used when `STOA.md` doesn't override.
    #[must_use]
    pub fn defaults() -> &'static [Self] {
        &[Self::Entity, Self::Concept, Self::Synthesis]
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Page lifecycle status (ARCHITECTURE §2 + §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Currently authoritative.
    Active,
    /// Replaced by a newer page (see `supersedes:` link).
    Superseded,
    /// Heuristically flagged as no-longer-fresh (ARCHITECTURE §4.2).
    Stale,
    /// Hard-retired by user/agent.
    Deprecated,
}

impl Status {
    /// Canonical lowercase string for the status.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Stale => "stale",
            Self::Deprecated => "deprecated",
        }
    }

    /// Default allow-list (ARCHITECTURE §2).
    #[must_use]
    pub fn defaults() -> &'static [Self] {
        &[
            Self::Active,
            Self::Superseded,
            Self::Stale,
            Self::Deprecated,
        ]
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
