//! Default `STOA.md` template emitted by `stoa init`.
//!
//! The schema is the most important file in a Stoa workspace (ARCHITECTURE
//! §3) — it's loaded into every agent context that touches the wiki. The
//! defaults here mirror §5 (entity types + relationship types).

/// Default STOA.md contents — written by `stoa init` and **only** if no
/// `STOA.md` already exists at the workspace root.
pub(crate) const DEFAULT_STOA_MD: &str = include_str!("./templates/STOA.default.md");
