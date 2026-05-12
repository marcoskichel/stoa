//! Stoa default recall backend: `ChromaDB` + `SQLite` FTS5 + KG.
//!
//! M1 skeleton — concrete API lands in M4 (Recall + `LocalChromaSqliteBackend`).
//! In v0.1 the embedding side calls into the Python sidecar (`stoa-embed`).

/// Crate version, sourced from `Cargo.toml` at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
