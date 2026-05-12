//! Page-id parsing + routing (ARCHITECTURE §2).
//!
//! Stoa page ids are slug-style and carry a stable prefix that maps directly
//! to a wiki sub-directory:
//!
//! | Prefix | Directory       |
//! |--------|-----------------|
//! | `ent-` | `wiki/entities/` |
//! | `con-` | `wiki/concepts/` |
//! | `syn-` | `wiki/synthesis/` |
//!
//! This prefix→dir mapping is the only routing rule in the system.

/// One of the three canonical wiki sub-directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageDir {
    /// `wiki/entities/`
    Entities,
    /// `wiki/concepts/`
    Concepts,
    /// `wiki/synthesis/`
    Synthesis,
}

impl PageDir {
    /// Relative sub-directory name under `wiki/`.
    #[must_use]
    pub fn as_subdir(self) -> &'static str {
        match self {
            Self::Entities => "entities",
            Self::Concepts => "concepts",
            Self::Synthesis => "synthesis",
        }
    }

    /// All three directories in canonical order.
    #[must_use]
    pub fn all() -> [Self; 3] {
        [Self::Entities, Self::Concepts, Self::Synthesis]
    }
}

/// A wiki page id with its routing dir resolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id {
    /// Full id including the prefix (e.g. `ent-redis`).
    pub raw: String,
    /// Directory derived from the id prefix.
    pub dir: PageDir,
}

impl Id {
    /// Parse an id, classifying by its three-letter prefix. Returns `None`
    /// for unknown prefixes — callers decide whether that's an error.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let dir = Self::dir_for(raw)?;
        Some(Self { raw: raw.to_owned(), dir })
    }

    /// Map an id prefix to its canonical [`PageDir`].
    #[must_use]
    pub fn dir_for(raw: &str) -> Option<PageDir> {
        match raw {
            r if r.starts_with("ent-") => Some(PageDir::Entities),
            r if r.starts_with("con-") => Some(PageDir::Concepts),
            r if r.starts_with("syn-") => Some(PageDir::Synthesis),
            _ => None,
        }
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, PageDir};

    #[test]
    fn parses_entity_prefix() {
        let id = Id::parse("ent-redis").unwrap();
        assert_eq!(id.dir, PageDir::Entities);
    }

    #[test]
    fn parses_concept_prefix() {
        let id = Id::parse("con-rag").unwrap();
        assert_eq!(id.dir, PageDir::Concepts);
    }

    #[test]
    fn parses_synthesis_prefix() {
        let id = Id::parse("syn-x").unwrap();
        assert_eq!(id.dir, PageDir::Synthesis);
    }

    #[test]
    fn rejects_unknown_prefix() {
        assert!(Id::parse("xxx-broken").is_none());
    }
}
