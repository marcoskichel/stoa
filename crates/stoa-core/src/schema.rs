//! In-memory model of the workspace `STOA.md` schema (ARCHITECTURE §3).
//!
//! For M2 the schema's job is narrow: hold the allow-lists used by
//! [`crate::validate_page`]. The default vocabulary ships with `stoa init`.
//! Parsing pulls extra entries the user added; unknown tokens stay opt-in
//! per workspace.

use std::collections::BTreeSet;

use crate::kind::{Kind, Status};
use crate::relationship::{DEFAULT_ENTITY_TYPES, DEFAULT_RELATIONSHIP_TYPES};

/// Workspace-scoped vocabulary backing schema validation.
///
/// Stored as sorted sets so `entity_types()` / `relationship_types()` output
/// is deterministic — useful when surfacing them in error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    entity_types: BTreeSet<String>,
    relationship_types: BTreeSet<String>,
    kinds: BTreeSet<String>,
    statuses: BTreeSet<String>,
}

impl Schema {
    /// Default schema (ARCHITECTURE §5). Used when no `STOA.md` is on disk
    /// or as the fallback floor for [`Schema::from_stoa_md`].
    #[must_use]
    pub fn defaults() -> Self {
        let entity_types = DEFAULT_ENTITY_TYPES
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let relationship_types = DEFAULT_RELATIONSHIP_TYPES
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let kinds = Kind::defaults()
            .iter()
            .map(|k| k.as_str().to_owned())
            .collect();
        let statuses = Status::defaults()
            .iter()
            .map(|s| s.as_str().to_owned())
            .collect();
        Self {
            entity_types,
            relationship_types,
            kinds,
            statuses,
        }
    }

    /// Build a schema from a `STOA.md` document. Starts from defaults and
    /// extends with any extra vocabulary mentioned in the file.
    ///
    /// The parser is intentionally forgiving — `STOA.md` is human-edited
    /// markdown, not strict YAML. We scan for fenced-code or bullet-list
    /// entries under "Entity types" / "Relationship types" headings.
    #[must_use]
    pub fn from_stoa_md(text: &str) -> Self {
        let mut schema = Self::defaults();
        let mut section: Option<Section> = None;
        for raw_line in text.lines() {
            let line = raw_line.trim_start();
            if let Some(next) = Section::detect(line) {
                section = Some(next);
                continue;
            }
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') {
                section = None;
                continue;
            }
            if let Some(token) = parse_bullet_token(line) {
                schema.add(section, token);
            }
        }
        schema
    }

    fn add(&mut self, section: Option<Section>, token: String) {
        match section {
            Some(Section::EntityTypes) => {
                let _ignored = self.entity_types.insert(token);
            },
            Some(Section::RelationshipTypes) => {
                let _ignored = self.relationship_types.insert(token);
            },
            _ => {},
        }
    }

    /// Sorted view of the entity-type allow-list.
    #[must_use]
    pub fn entity_types(&self) -> Vec<&str> {
        self.entity_types.iter().map(String::as_str).collect()
    }

    /// Sorted view of the relationship-type allow-list.
    #[must_use]
    pub fn relationship_types(&self) -> Vec<&str> {
        self.relationship_types.iter().map(String::as_str).collect()
    }

    /// True if `value` is a recognized entity-type.
    #[must_use]
    pub fn allows_entity_type(&self, value: &str) -> bool {
        self.entity_types.contains(value)
    }

    /// True if `value` is a recognized relationship-type.
    #[must_use]
    pub fn allows_relationship_type(&self, value: &str) -> bool {
        self.relationship_types.contains(value)
    }

    /// True if `value` is a recognized page-kind.
    #[must_use]
    pub fn allows_kind(&self, value: &str) -> bool {
        self.kinds.contains(value)
    }

    /// True if `value` is a recognized status.
    #[must_use]
    pub fn allows_status(&self, value: &str) -> bool {
        self.statuses.contains(value)
    }
}

#[derive(Debug, Clone, Copy)]
enum Section {
    EntityTypes,
    RelationshipTypes,
}

impl Section {
    fn detect(line: &str) -> Option<Self> {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with('#') && lower.contains("entity") && lower.contains("type") {
            Some(Self::EntityTypes)
        } else if lower.starts_with('#') && lower.contains("relationship") {
            Some(Self::RelationshipTypes)
        } else {
            None
        }
    }
}

fn parse_bullet_token(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let body = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))?;
    // NOTE: prefer the first backtick-quoted token over a bare word so prose around it is ignored.
    if let Some(rest) = body.strip_prefix('`') {
        let end = rest.find('`')?;
        return Some(rest[..end].to_owned());
    }
    body.split_whitespace().next().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::Schema;

    #[test]
    fn defaults_cover_documented_types() {
        let s = Schema::defaults();
        assert!(s.allows_entity_type("library"));
        assert!(s.allows_entity_type("decision"));
        assert!(s.allows_relationship_type("depends_on"));
        assert!(s.allows_relationship_type("supersedes"));
        assert!(s.allows_kind("entity"));
        assert!(s.allows_status("active"));
    }

    #[test]
    fn parses_extra_entity_type_from_md() {
        let md = "# Entity types\n- `widget` — a widget thing\n";
        let s = Schema::from_stoa_md(md);
        assert!(s.allows_entity_type("widget"));
        assert!(s.allows_entity_type("library"), "defaults preserved");
    }

    #[test]
    fn parses_extra_relationship_type_from_md() {
        let md = "# Relationship types\n- `blocks` — A blocks B\n";
        let s = Schema::from_stoa_md(md);
        assert!(s.allows_relationship_type("blocks"));
        assert!(s.allows_relationship_type("depends_on"), "defaults preserved");
    }
}
