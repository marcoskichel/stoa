//! FTS5 query sanitizer.
//!
//! FTS5 treats `" * ( ) :` as operators and the bare keyword `NEAR` as
//! a proximity operator. Stoa's recall surface is a literal keyword
//! search, so we wrap each token in double quotes (doubling embedded
//! `"` per the FTS5 quoting rules) and join them with spaces — the
//! FTS5 default, which is AND.
//!
//! AND-by-default is the precise behavior callers expect from a
//! workspace search. The previous OR-join silently widened recall.
//! When a query DSL ships callers can opt back into OR / NEAR; until
//! then, this module is the single place that decides.

/// Quote every token in `q` so FTS5 treats them as literals; join the
/// resulting tokens with spaces (AND).
///
/// Returns the empty string when no tokens survive (e.g. pure
/// punctuation input). Empty / whitespace-only input also returns the
/// empty string so the caller can short-circuit before issuing a
/// MATCH that FTS5 would reject.
pub(crate) fn sanitize_query(q: &str) -> String {
    let trimmed = q.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for token in trimmed.split_whitespace() {
        if let Some(safe) = sanitize_one_token(token) {
            parts.push(safe);
        }
    }
    parts.join(" ")
}

/// Quote `token` for FTS5 literal interpretation. Returns `None` if
/// the token contains no FTS5-tokenizable codepoint (digits or
/// alphabetic) — e.g. pure punctuation like `:` or `^`.
fn sanitize_one_token(token: &str) -> Option<String> {
    if !token.chars().any(char::is_alphanumeric) {
        return None;
    }
    let cleaned = token.replace('"', "\"\"");
    Some(format!("\"{cleaned}\""))
}

#[cfg(test)]
mod tests {
    use super::sanitize_query;

    #[test]
    fn default_is_and() {
        let q = sanitize_query("redis cache");
        assert_eq!(q, "\"redis\" \"cache\"", "default join must be space (FTS5 AND)");
    }

    #[test]
    fn skips_pure_punctuation_tokens() {
        for token in [":", "^", "*", "(", ")"] {
            let q = sanitize_query(&format!("redis {token} cache"));
            assert!(!q.contains(token), "punctuation `{token}` must be stripped: {q}");
            assert!(q.contains("redis") && q.contains("cache"));
        }
    }

    #[test]
    fn quotes_near_keyword_literal() {
        let q = sanitize_query("NEAR foo");
        assert!(q.contains("\"NEAR\""), "literal `NEAR` must survive as quoted token: {q}");
    }

    #[test]
    fn doubles_embedded_double_quotes() {
        let q = sanitize_query(r#"a"b"#);
        assert_eq!(q, "\"a\"\"b\"", "embedded `\"` must be doubled per FTS5 rules");
    }

    #[test]
    fn empty_yields_empty() {
        assert_eq!(sanitize_query(""), "");
        assert_eq!(sanitize_query("   "), "");
        assert_eq!(sanitize_query(": ^ *"), "");
    }
}
