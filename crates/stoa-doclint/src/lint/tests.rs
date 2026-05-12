//! Unit tests for the trivial-doc heuristic.

use super::tokens::{FillerSet, doc_signal_tokens, extract_words};

#[test]
fn extract_words_splits_snake_camel_screaming() {
    let toks = extract_words("CARGO_PKG_VERSION");
    assert!(toks.contains("cargo"));
    assert!(toks.contains("pkg"));
    assert!(toks.contains("version"));
    let toks = extract_words("HttpClientBuilder");
    assert!(toks.contains("http"));
    assert!(toks.contains("client"));
    assert!(toks.contains("builder"));
}

#[test]
fn extract_words_drops_single_chars() {
    let toks = extract_words("a_b_long_name");
    assert!(!toks.contains("a"));
    assert!(!toks.contains("b"));
    assert!(toks.contains("long"));
    assert!(toks.contains("name"));
}

#[test]
fn doc_residue_strips_filler_and_glue() {
    let filler = FillerSet::default();
    let residue =
        doc_signal_tokens("Crate version, sourced from `Cargo.toml` at build time.", &filler);
    assert_eq!(residue.len(), 1);
    assert!(residue.contains("version"));
}

#[test]
fn doc_residue_keeps_semantic_words() {
    let filler = FillerSet::default();
    let residue = doc_signal_tokens(
        "Crate version, used as the canonical id in Workspace::register.",
        &filler,
    );
    assert!(residue.contains("version"));
    assert!(residue.contains("canonical"));
    assert!(residue.contains("workspace"));
    assert!(residue.contains("register"));
}
