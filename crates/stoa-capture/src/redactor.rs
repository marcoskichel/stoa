//! Line-level redaction engine.
//!
//! Compiles the [`crate::patterns::DEFAULTS`] catalogue into a vector of
//! `(kind, Regex, replacement)` once and applies them in order on every
//! line. Earlier patterns are more specific (e.g. `anthropic` before
//! `openai`) so the correct kind label is preferred when ranges overlap.
//!
//! Hot-path discipline: a `RegexSet` prefilter rejects lines that contain
//! no secrets without ever allocating; only lines that match at least one
//! pattern walk the per-rule replace loop.

use std::borrow::Cow;

use regex::{Regex, RegexSet};

use crate::patterns::{DEFAULTS, Pattern};

/// Compiled redactor with a fixed pattern set.
#[derive(Debug)]
pub struct Redactor {
    rules: Vec<Rule>,
    prefilter: RegexSet,
}

#[derive(Debug)]
struct Rule {
    re: Regex,
    /// Pre-rendered `[REDACTED:<kind>]` replacement (allocated once on build).
    replacement: String,
}

impl Redactor {
    /// Build a redactor with the default pattern set (per ARCHITECTURE §10).
    ///
    /// Compiles every pattern + a single [`RegexSet`] prefilter; the
    /// prefilter is `O(line length)` and rejects clean lines without
    /// allocating, which is the common case on captured transcripts.
    pub fn with_defaults() -> Self {
        let rules: Vec<Rule> = DEFAULTS.iter().filter_map(compile_rule).collect();
        let prefilter =
            RegexSet::new(DEFAULTS.iter().map(|p| p.regex)).unwrap_or_else(|_| RegexSet::empty());
        Self { rules, prefilter }
    }

    /// Redact one line, returning the rewritten string. Idempotent: a
    /// redacted line is a no-op on a second pass.
    ///
    /// Returns the original line via `to_owned()` when no rule matches —
    /// callers receive an owned `String` either way to keep the API
    /// stable, but the per-rule replace loop is skipped on clean lines.
    pub fn redact_line(&self, line: &str) -> String {
        if !self.prefilter.is_match(line) {
            return line.to_owned();
        }
        let mut out: Cow<'_, str> = Cow::Borrowed(line);
        for rule in &self.rules {
            let replaced = rule.re.replace_all(&out, rule.replacement.as_str());
            if let Cow::Owned(s) = replaced {
                out = Cow::Owned(s);
            }
        }
        out.into_owned()
    }
}

fn compile_rule(p: &Pattern) -> Option<Rule> {
    Regex::new(p.regex).ok().map(|re| Rule {
        re,
        replacement: format!("[REDACTED:{}]", p.kind),
    })
}
