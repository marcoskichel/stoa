//! Line-level redaction engine.
//!
//! Compiles the [`crate::patterns::DEFAULTS`] catalogue into a vector of
//! `(kind, Regex)` once and applies them in order on every line. Earlier
//! patterns are more specific (e.g. `anthropic` before `openai`) so the
//! correct kind label is preferred when ranges overlap.

use regex::Regex;

use crate::patterns::{DEFAULTS, Pattern};

/// Compiled redactor with a fixed pattern set.
#[derive(Debug)]
pub struct Redactor {
    rules: Vec<Rule>,
}

#[derive(Debug)]
struct Rule {
    kind: &'static str,
    re: Regex,
}

impl Redactor {
    /// Build a redactor with the default pattern set (per ARCHITECTURE §10).
    pub fn with_defaults() -> Self {
        let rules = DEFAULTS.iter().filter_map(compile_rule).collect();
        Self { rules }
    }

    /// Redact one line, returning the rewritten string. Idempotent: a
    /// redacted line is a no-op on a second pass.
    pub fn redact_line(&self, line: &str) -> String {
        let mut out = line.to_owned();
        for rule in &self.rules {
            out = rule
                .re
                .replace_all(&out, replacement(rule.kind))
                .into_owned();
        }
        out
    }
}

fn compile_rule(p: &Pattern) -> Option<Rule> {
    Regex::new(p.regex).ok().map(|re| Rule { kind: p.kind, re })
}

fn replacement(kind: &str) -> String {
    format!("[REDACTED:{kind}]")
}
