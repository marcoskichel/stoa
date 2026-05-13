//! Default redaction patterns sourced from gitleaks + secrets-patterns-db.
//!
//! See [ARCHITECTURE.md §10] — the redaction filter runs before any content
//! reaches durable storage. Patterns are hand-vendored from gitleaks
//! (MIT-licensed), the secrets-patterns-db catalogue, and the `AgentLeak`
//! benchmark's synthetic-PII canary scheme.
//!
//! Every entry returns a `<kind>` string used to label the replacement:
//! `[REDACTED:<kind>]`. Test contract requires substring match on these
//! `<kind>` names — e.g. `aws`, `email`, `github-pat`, `jwt`, `path`,
//! `ssn-us`, `phone-us`, `credit-card`, `iban`, `ipv4`, `mac-address`,
//! `canary-token`.

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
///
/// Layout:
/// 1. Cloud + provider credentials (AWS, Anthropic, GitHub, GitLab, Slack,
///    Stripe, `OpenAI`, generic JWT, generic Bearer).
/// 2. Email + on-disk secret-store paths.
/// 3. Structured PII (SSN, phone, credit card, IBAN).
/// 4. Network identifiers (IPv4, MAC address).
/// 5. `AgentLeak` synthetic-PII canary tokens (placed last so structured PII
///    is preferred when a canary happens to look like one).
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
    // NOTE: `ssn-us` — US Social Security Number `XXX-XX-XXXX`. The strict
    // NOTE: 3-2-4 hyphen layout makes false-positives on phone numbers
    // NOTE: (3-4 or 3-3-4) impossible without re-ordering rules.
    Pattern {
        kind: "ssn-us",
        regex: r"\b\d{3}-\d{2}-\d{4}\b",
    },
    // NOTE: `phone-us-10d` — 10-digit US phone with optional area parens
    // NOTE: and `-`, ` `, or `.` separators. Word-boundary anchored so it
    // NOTE: does not chew through unrelated digit runs.
    Pattern {
        kind: "phone-us-10d",
        regex: r"\b\(?\d{3}\)?[\s\.\-]\d{3}[\s\.\-]\d{4}\b",
    },
    // NOTE: `phone-us-7d` — 7-digit local form `XXX-XXXX` used in synthetic
    // NOTE: test fixtures (`555-0174` in AgentLeak). After the 10-digit rule
    // NOTE: so the longer match wins on overlap.
    Pattern {
        kind: "phone-us-7d",
        regex: r"\b\d{3}-\d{4}\b",
    },
    // NOTE: `credit-card` — 16-digit PAN in the canonical `XXXX-XXXX-XXXX-XXXX`
    // NOTE: form (with `-`, space, or no separator). 13/19-digit Amex/Maestro
    // NOTE: variants are deliberately excluded to keep false-positives low.
    Pattern {
        kind: "credit-card",
        regex: r"\b(?:\d{4}[\s\-]?){3}\d{4}\b",
    },
    // NOTE: `iban` — country code (2 letters) + check digits (2) + 1-7 groups
    // NOTE: of four alphanumerics (with optional spaces between groups).
    Pattern {
        kind: "iban",
        regex: r"\b[A-Z]{2}\d{2}(?:[\s]?[A-Z0-9]{4}){1,7}\b",
    },
    // NOTE: `ipv4` — dotted-decimal with strict per-octet 0..=255 ranges so
    // NOTE: version strings like `1.2.3` do not trigger a false positive.
    Pattern {
        kind: "ipv4",
        regex: r"\b(?:25[0-5]|2[0-4]\d|1?\d{1,2})(?:\.(?:25[0-5]|2[0-4]\d|1?\d{1,2})){3}\b",
    },
    // NOTE: `mac-address` — six hex pairs separated by `:` or `-`,
    // NOTE: case-insensitive so both `00:1B:44:11:3A:B7` and
    // NOTE: `00-1b-44-11-3a-b7` match without a second rule.
    Pattern {
        kind: "mac-address",
        regex: r"(?i)\b(?:[0-9A-F]{2}[:\-]){5}[0-9A-F]{2}\b",
    },
    // NOTE: `canary-token` — AgentLeak's synthetic-PII shape
    // NOTE: `CANARY_<KIND>_<ID>` (e.g. `CANARY_SSN_ZVR5XO4K`). Placed last so
    // NOTE: structured PII rules still get the canonical label when they
    // NOTE: happen to overlap.
    Pattern {
        kind: "canary-token",
        regex: r"\bCANARY_[A-Z_]+_[A-Z0-9]+\b",
    },
];
