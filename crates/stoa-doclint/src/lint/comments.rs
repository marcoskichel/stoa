//! Comment-token scanner.
//!
//! Walks `rustc_lexer` tokens over a source file and flags every line/block
//! comment that is neither a doc comment (`///`, `//!`, `/** */`, `/*! */`)
//! nor a line comment opening with one of [`ALLOWED_PREFIXES`].
//!
//! Doc-style detection works by slicing the comment text — `rustc_lexer
//! 0.1.0`'s `TokenKind::LineComment` / `BlockComment` does not carry a
//! `doc_style` field, but the leading byte pattern is unambiguous per the
//! Rust reference's comment grammar.

use std::fmt;
use std::path::{Path, PathBuf};

use rustc_lexer::{Token, TokenKind, tokenize};

/// Intent prefixes that justify a bare `//` line comment.
///
/// Curated short list. `TODO:` is intentionally absent — TODOs decay; route
/// them through the issue tracker so they have an owner. Each accepted prefix
/// labels a *durable* reason the comment must exist:
///
/// - `SAFETY:` documents the precondition that justifies an `unsafe` block.
/// - `FIXME:` flags a known wrong-but-deliberate compromise.
/// - `HACK:`  flags a workaround for an external bug.
/// - `PERF:`  records a measured choice that looks unusual.
/// - `NOTE:`  surfaces a non-obvious invariant a reader would miss.
/// - `WHY:`   justifies a design call when the code alone is ambiguous.
pub(crate) const ALLOWED_PREFIXES: &[&str] =
    &["SAFETY:", "FIXME:", "HACK:", "PERF:", "NOTE:", "WHY:"];

/// One forbidden-comment finding.
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
    pub(crate) kind: Kind,
    pub(crate) snippet: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    BareLine,
    BareBlock,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hint = match self.kind {
            Kind::BareLine => {
                "use `///`/`//!` doc, or prefix `// SAFETY:`/`FIXME:`/`HACK:`/`PERF:`/`NOTE:`/`WHY:`"
            },
            Kind::BareBlock => {
                "non-doc block comments are forbidden; use `///`/`//!` or `/** */`/`/*! */`"
            },
        };
        write!(
            f,
            "{}:{}: forbidden comment — {} (saw: {:?})",
            self.path.display(),
            self.line,
            hint,
            self.snippet,
        )
    }
}

/// Run the lexer over `src` and emit findings for every bare comment.
pub(crate) fn check_source(path: &Path, src: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut line = 1usize;
    for Token { kind, len } in tokenize(src) {
        let end = offset + len;
        let slice = &src[offset..end];
        if let Some(kind) = classify(slice, kind) {
            out.push(Finding {
                path: path.to_path_buf(),
                line,
                kind,
                snippet: snippet(slice),
            });
        }
        line += slice.bytes().filter(|byte| *byte == b'\n').count();
        offset = end;
    }
    out
}

fn classify(slice: &str, kind: TokenKind) -> Option<Kind> {
    match kind {
        TokenKind::LineComment if !is_doc_line(slice) && !has_allowed_prefix(slice) => {
            Some(Kind::BareLine)
        },
        TokenKind::BlockComment { .. } if !is_doc_block(slice) => Some(Kind::BareBlock),
        _ => None,
    }
}

fn is_doc_line(slice: &str) -> bool {
    slice.starts_with("///") || slice.starts_with("//!")
}

/// Rust reference: `/** ... */` is outer doc and `/*! ... */` is inner doc,
/// **except** when the content after the opener is empty or made entirely of
/// `*` characters — those collapse to plain block comments. The simplified
/// check below accepts a block as doc only when the byte after `/**` is not a
/// `*` and not `/`, which agrees with the spec for every shape that appears
/// in normal source.
fn is_doc_block(slice: &str) -> bool {
    if slice.starts_with("/*!") {
        return true;
    }
    let Some(rest) = slice.strip_prefix("/**") else {
        return false;
    };
    !(rest.is_empty() || rest.starts_with('*') || rest.starts_with('/'))
}

fn has_allowed_prefix(line_comment: &str) -> bool {
    let body = line_comment.trim_start_matches('/').trim_start();
    ALLOWED_PREFIXES.iter().any(|prefix| body.starts_with(prefix))
}

fn snippet(slice: &str) -> String {
    let trimmed = slice.trim();
    if trimmed.len() > 80 {
        format!("{}...", &trimmed[..80])
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{has_allowed_prefix, is_doc_block, is_doc_line};

    #[test]
    fn doc_line_detection() {
        assert!(is_doc_line("/// outer"));
        assert!(is_doc_line("//! inner"));
        assert!(!is_doc_line("// bare"));
        assert!(!is_doc_line("//SAFETY: x"));
    }

    #[test]
    fn doc_block_detection() {
        assert!(is_doc_block("/** outer */"));
        assert!(is_doc_block("/*! inner */"));
        assert!(!is_doc_block("/* plain */"));
        assert!(!is_doc_block("/**/"));
        assert!(!is_doc_block("/***/"));
        assert!(!is_doc_block("/****/"));
    }

    #[test]
    fn allowed_prefix_accepts_all_six() {
        for prefix in ["SAFETY:", "FIXME:", "HACK:", "PERF:", "NOTE:", "WHY:"] {
            assert!(has_allowed_prefix(&format!("// {prefix} reason")));
            assert!(has_allowed_prefix(&format!("//{prefix} reason")));
        }
    }

    #[test]
    fn allowed_prefix_rejects_todo_and_bare() {
        assert!(!has_allowed_prefix("// TODO: not allowed"));
        assert!(!has_allowed_prefix("// plain note"));
        assert!(!has_allowed_prefix("// safety: lowercase loses the contract"));
    }
}
