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
///
/// Per-entry intent (one line each):
/// - `aws`: `AKIA|ASIA|AROA|AIDA|ANPA|ANVA|AGPA` + 16 base32 chars.
/// - `anthropic`: `sk-ant-(api|sid|admin)<digits>-<32..200 url-safe>`.
/// - `github-pat`: `ghp_` + 36+ base62 chars.
/// - `github-oauth`: `gho_` + 36+ base62 chars.
/// - `github-app`: `ghu_` or `ghs_` + 36+ base62 chars.
/// - `github-refresh`: `ghr_` + 36+ base62 chars.
/// - `gitlab`: `glpat-` + 20+ url-safe chars.
/// - `slack`: `xox[abprs]-` + 10+ chars.
/// - `stripe-live` / `stripe-test`: `sk_(live|test)_` + 24+ base62 chars.
/// - `stripe-restricted`: `rk_(live|test)_` + 24+ base62 chars.
/// - `jwt`: three base64url chunks separated by `.`, each ≥10 chars.
/// - `openai`: `sk-(proj|svcacct|admin)-` + 20+ url-safe chars.
/// - `bearer`: case-insensitive `Bearer <token>` with 20+ chars of body.
/// - `email`: RFC-5322-ish local-part `@` domain with TLD ≥2.
/// - `path-{ssh,aws,gnupg}`: `~/.<store>` or `<...>/.<store>/<file>`.
/// - `ssn-us`: `XXX-XX-XXXX` (strict 3-2-4 hyphen layout).
/// - `phone-us-10d`: 10-digit US phone with optional area parens and
///   `-`, ` `, or `.` separators; word-boundary anchored. The previous
///   7-digit variant (`XXX-XXXX`) was removed because it over-matched
///   HTTP-style `404-5230`, ZIP+4 fragments, and version strings.
/// - `credit-card`: 16-digit PAN with mandatory `-` or space between
///   every group; unseparated 16-digit runs are skipped.
/// - `iban`: ISO 3166 alpha-2 + 2 check digits + 3..7 groups of four
///   alphanumerics + an optional trailing 1..3-char partial group.
/// - `ipv4`: dotted-decimal with strict per-octet 0..=255 ranges so
///   version strings like `1.2.3` do not match.
/// - `mac-address`: six hex pairs separated by `:` or `-`,
///   case-insensitive.
/// - `canary-token`: `AgentLeak`'s `CANARY_<KIND>_<ID>` shape; placed
///   last so structured PII rules win on overlap.
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
    Pattern {
        kind: "ssn-us",
        regex: r"\b\d{3}-\d{2}-\d{4}\b",
    },
    Pattern {
        kind: "phone-us-10d",
        regex: r"\b\(?\d{3}\)?[\s\.\-]\d{3}[\s\.\-]\d{4}\b",
    },
    Pattern {
        kind: "credit-card",
        regex: r"\b\d{4}[\s\-]\d{4}[\s\-]\d{4}[\s\-]\d{4}\b",
    },
    Pattern {
        kind: "iban",
        regex: r"\b[A-Z]{2}\d{2}(?:\s?[A-Z0-9]{4}){3,7}(?:\s?[A-Z0-9]{1,3})?\b",
    },
    Pattern {
        kind: "ipv4",
        regex: r"\b(?:25[0-5]|2[0-4]\d|1?\d{1,2})(?:\.(?:25[0-5]|2[0-4]\d|1?\d{1,2})){3}\b",
    },
    Pattern {
        kind: "mac-address",
        regex: r"(?i)\b(?:[0-9A-F]{2}[:\-]){5}[0-9A-F]{2}\b",
    },
    Pattern {
        kind: "canary-token",
        regex: r"\bCANARY_[A-Z_]+_[A-Z0-9]+\b",
    },
];
