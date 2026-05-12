//! Stoa default recall backend: `ChromaDB` + `SQLite` FTS5 + KG.
//!
//! M1 skeleton — concrete API lands in M4 (Recall + `LocalChromaSqliteBackend`).
//! In v0.1 the embedding side calls into the Python sidecar (`stoa-embed`).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_version_is_not_empty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
