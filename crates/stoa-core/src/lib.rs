//! Stoa core types: frontmatter, ids, schema, validation.
//!
//! M2 — Wiki + CLI core. The concrete API consumed by `stoa-cli` and the
//! frontmatter property tests lives here.
//!
//! Spec source: [ARCHITECTURE.md §2 Wiki data model] + [§3 Schema] + [§5 KG].

pub mod error;
pub mod frontmatter;
pub mod id;
pub mod kind;
pub mod relationship;
pub mod schema;
pub mod validate;

pub use error::{Error, Result, ValidationError};
pub use frontmatter::Frontmatter;
pub use id::{Id, PageDir, SessionId};
pub use kind::{Kind, Status};
pub use relationship::{EntityType, Relationship, RelationshipType};
pub use schema::Schema;
pub use validate::validate_page;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_version_is_not_empty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
