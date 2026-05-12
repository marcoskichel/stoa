//! Stoa capture worker (transcript drain + PII redaction).
//!
//! M1 skeleton — concrete API lands in M3 (Capture pipeline).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_version_is_not_empty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
