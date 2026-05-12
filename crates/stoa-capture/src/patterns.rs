//! Default redaction patterns sourced from gitleaks + secrets-patterns-db.
//!
//! See [ARCHITECTURE.md §10] — the redaction filter runs before any content
//! reaches durable storage. Patterns are hand-vendored from gitleaks
//! (MIT-licensed) and the secrets-patterns-db catalogue.
//!
//! Every entry returns a `<kind>` string used to label the replacement:
//! `[REDACTED:<kind>]`. Test contract requires substring match on these
//! `<kind>` names — e.g. `aws`, `email`, `github-pat`, `jwt`, `path`.

/// One named regex pattern.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Pattern {
    /// Short tag inserted into `[REDACTED:<kind>]`.
    pub(crate) kind: &'static str,
    /// Regex source (compiled once on [`crate::Redactor::with_defaults`]).
    pub(crate) regex: &'static str,
}

/// Pattern catalogue. Order matters when patterns overlap — the longest /
/// most specific match should run first.
pub(crate) const DEFAULTS: &[Pattern] = &[
    Pattern {
        kind: "aws",
        regex: r"\b(?:AKIA|ASIA|AROA|AIDA|ANPA|ANVA|AGPA)[A-Z0-9]{16}\b",
    },
    Pattern {
        kind: "anthropic",
        regex: r"\bsk-ant-(?:api|sid|admin)\d*-[A-Za-z0-9_\-]{32,200}\b",
    },
    Pattern {
        kind: "github-pat",
        regex: r"\bghp_[A-Za-z0-9]{36,}\b",
    },
    Pattern {
        kind: "github-oauth",
        regex: r"\bgho_[A-Za-z0-9]{36,}\b",
    },
    Pattern {
        kind: "github-app",
        regex: r"\b(?:ghu|ghs)_[A-Za-z0-9]{36,}\b",
    },
    Pattern {
        kind: "github-refresh",
        regex: r"\bghr_[A-Za-z0-9]{36,}\b",
    },
    Pattern {
        kind: "gitlab",
        regex: r"\bglpat-[A-Za-z0-9_\-]{20,}\b",
    },
    Pattern {
        kind: "slack",
        regex: r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b",
    },
    Pattern {
        kind: "stripe-live",
        regex: r"\bsk_live_[A-Za-z0-9]{24,}\b",
    },
    Pattern {
        kind: "stripe-test",
        regex: r"\bsk_test_[A-Za-z0-9]{24,}\b",
    },
    Pattern {
        kind: "stripe-restricted",
        regex: r"\brk_(?:live|test)_[A-Za-z0-9]{24,}\b",
    },
    Pattern {
        kind: "jwt",
        regex: r"\beyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\b",
    },
    Pattern {
        kind: "openai",
        regex: r"\bsk-(?:proj|svcacct|admin)-[A-Za-z0-9_\-]{20,200}\b",
    },
    Pattern {
        kind: "bearer",
        regex: r"(?i)\bBearer\s+[A-Za-z0-9_\-\.=]{20,}\b",
    },
    Pattern {
        kind: "email",
        regex: r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b",
    },
    Pattern {
        kind: "path-ssh",
        regex: r#"(?:[A-Za-z]?[/\\][^\s"',]*[/\\]|~[/\\])\.ssh(?:[/\\][^\s"',]+)?"#,
    },
    Pattern {
        kind: "path-aws",
        regex: r#"(?:[A-Za-z]?[/\\][^\s"',]*[/\\]|~[/\\])\.aws(?:[/\\][^\s"',]+)?"#,
    },
    Pattern {
        kind: "path-gnupg",
        regex: r#"(?:[A-Za-z]?[/\\][^\s"',]*[/\\]|~[/\\])\.gnupg(?:[/\\][^\s"',]+)?"#,
    },
];
