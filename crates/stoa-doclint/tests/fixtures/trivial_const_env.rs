//! Fixture: trivial doc on `const VERSION = env!(...)`. Must be flagged.
//!
//! NOTE: this file is intentionally not part of any crate. It is read as a
//! plain source file by `stoa-doclint` and never compiled. The `tests/`
//! directory is excluded from the lint's own self-scan via `is_excluded`,
//! so adding new fixtures here will not regress against the lint.

/// Crate version, sourced from `Cargo.toml` at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
