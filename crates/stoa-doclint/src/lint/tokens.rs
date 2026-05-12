//! Word-splitting + stopword/filler filtering for identifiers and prose.

use std::collections::HashSet;

/// Split a string into lowercased word tokens, honoring `snake_case`,
/// `camelCase`, `SCREAMING_SNAKE`, and arbitrary punctuation as boundaries.
///
/// Single-character words (e.g. `a`) are dropped — they are never load-bearing
/// signal in either identifiers or prose.
pub(crate) fn extract_words(s: &str) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    let mut buf = String::new();
    let mut prev_lower = false;
    for ch in s.chars() {
        if !ch.is_ascii_alphanumeric() {
            flush(&mut out, &mut buf);
            prev_lower = false;
            continue;
        }
        if ch.is_ascii_uppercase() && prev_lower {
            flush(&mut out, &mut buf);
        }
        buf.push(ch.to_ascii_lowercase());
        prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }
    flush(&mut out, &mut buf);
    out.retain(|t| t.len() >= 2);
    out
}

fn flush(out: &mut HashSet<String>, buf: &mut String) {
    if !buf.is_empty() {
        out.insert(std::mem::take(buf));
    }
}

/// Tokens that carry no information regardless of context: English articles,
/// auxiliary verbs, and conjunctions; plus build/environment vocabulary that
/// is implied by `env!`-style macros and Cargo conventions.
///
/// The list is intentionally narrow. Wider would risk false negatives — a doc
/// that says "computed once at startup" is *not* trivial just because "once"
/// or "computed" sound generic.
pub(crate) struct FillerSet {
    words: HashSet<&'static str>,
}

impl Default for FillerSet {
    fn default() -> Self {
        Self {
            words: DEFAULT_FILLER.iter().copied().collect(),
        }
    }
}

impl FillerSet {
    fn contains(&self, w: &str) -> bool {
        self.words.contains(w)
    }
}

const DEFAULT_FILLER: &[&str] = &[
    // English glue
    "an",
    "the",
    "of",
    "in",
    "on",
    "at",
    "to",
    "for",
    "with",
    "by",
    "from",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "and",
    "or",
    "but",
    "not",
    "as",
    "this",
    "that",
    "these",
    "those",
    "it",
    "its",
    // Build / env-macro context — implied by `env!`, `option_env!`, etc.
    "crate",
    "package",
    "pkg",
    "cargo",
    "toml",
    "build",
    "compile",
    "compiled",
    "time",
    "env",
    "set",
    "value",
    "sourced",
    "via",
    "during",
    "construction",
];

/// Tokenize `doc`, strip stopwords + build-context filler, return the residue
/// — the words the doc actually contributes.
pub(crate) fn doc_signal_tokens(doc: &str, filler: &FillerSet) -> HashSet<String> {
    let mut t = extract_words(doc);
    t.retain(|w| !filler.contains(w));
    t
}
