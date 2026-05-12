//! Heuristic: a doc comment is "trivial" when every meaningful word in the
//! prose is already encoded in the identifier — after dropping articles and
//! common environment-macro filler (`crate`, `cargo`, `version`-context, etc).
//!
//! The intent is to catch tautologies like
//! `/// Crate version, sourced from Cargo.toml at build time.` sitting above
//! `pub const VERSION: &str = env!("CARGO_PKG_VERSION");` — the doc reads as
//! prose but adds nothing the identifier + `env!` macro do not already say.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::Visit;
use syn::{Attribute, Ident};
use walkdir::WalkDir;

mod tokens;
mod visitor;

use self::tokens::{FillerSet, doc_signal_tokens, extract_words};
use self::visitor::Visitor;

/// A single trivial-doc-comment finding.
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
    pub(crate) identifier: String,
    pub(crate) doc: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: trivial doc on `{}` adds no information beyond identifier (doc: {:?})",
            self.path.display(),
            self.line,
            self.identifier,
            self.doc,
        )
    }
}

/// Recursively scan `roots` and return any trivial-doc findings.
///
/// File arguments bypass the directory-exclusion filter; only `WalkDir`
/// traversals honor `is_excluded`. This lets callers pass a single fixture
/// file under `tests/fixtures/` without that fixture being silently skipped.
pub(crate) fn run(roots: &[PathBuf]) -> Vec<Finding> {
    let mut out = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_rust_source(root) {
                out.extend(check_file(root));
            }
            continue;
        }
        for entry in WalkDir::new(root).into_iter().flatten() {
            let p = entry.path();
            if is_rust_source(p) && !is_excluded(p) {
                out.extend(check_file(p));
            }
        }
    }
    out
}

fn is_rust_source(p: &Path) -> bool {
    p.is_file() && p.extension().is_some_and(|e| e == "rs")
}

fn is_excluded(p: &Path) -> bool {
    p.components()
        .any(|c| matches!(c.as_os_str().to_str(), Some("target" | "fixtures" | ".git"),))
}

fn check_file(path: &Path) -> Vec<Finding> {
    let Ok(src) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(ast) = syn::parse_file(&src) else {
        return Vec::new();
    };
    let mut v = Visitor::new(path);
    v.visit_file(&ast);
    v.into_findings()
}

pub(crate) fn check_one(path: &Path, attrs: &[Attribute], ident: &Ident) -> Option<Finding> {
    let (doc, line) = extract_doc(attrs)?;
    let ident_str = ident.to_string();
    let ident_toks = extract_words(&ident_str);
    let filler = FillerSet::default();
    let signal = doc_signal_tokens(&doc, &filler);
    if signal.is_empty() {
        return None;
    }
    if signal.iter().all(|t| ident_toks.contains(t)) {
        return Some(Finding {
            path: path.to_path_buf(),
            line,
            identifier: ident_str,
            doc,
        });
    }
    None
}

fn extract_doc(attrs: &[Attribute]) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut first_line = usize::MAX;
    for attr in attrs {
        let Some(piece) = doc_value(attr) else {
            continue;
        };
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(piece.trim());
        first_line = first_line.min(doc_line(attr));
    }
    if text.is_empty() {
        None
    } else {
        Some((text, first_line))
    }
}

fn doc_value(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }
    let syn::Meta::NameValue(nv) = &attr.meta else {
        return None;
    };
    let syn::Expr::Lit(lit) = &nv.value else {
        return None;
    };
    let syn::Lit::Str(s) = &lit.lit else {
        return None;
    };
    Some(s.value())
}

fn doc_line(attr: &Attribute) -> usize {
    let start = attr.pound_token.span.start();
    if start.line == 0 {
        usize::MAX
    } else {
        start.line
    }
}

#[cfg(test)]
mod tests;
