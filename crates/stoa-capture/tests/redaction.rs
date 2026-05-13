//! E2E quality gate: PII redaction patterns.
//!
//! Spec source: [ARCHITECTURE.md §10 Privacy, governance, adversarial defenses].
//!
//! Patterns sourced from gitleaks-config + secrets-patterns-db (per research).
//! Every required redaction class has a positive test (real-format example
//! → redacted) and at least one negative test (similar-looking but non-secret
//! → preserved).

mod common;

use common::{default_redactor, has_redaction_kind, has_redaction_marker};

#[test]
fn redacts_aws_access_key() {
    let r = default_redactor();
    let out = r.redact_line("export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE");
    assert!(has_redaction_kind(&out, "aws"), "AWS access key not redacted: {out:?}");
    assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn redacts_stripe_live_key() {
    let r = default_redactor();
    // NOTE: build the test fixture at runtime so the `sk_live_*` literal never
    // NOTE: appears in source. GitHub push-protection scans source files and flags
    // NOTE: any matching string regardless of synthetic entropy.
    let key = format!("sk_{}_{}", "live", "z".repeat(28));
    let out = r.redact_line(&format!("token={key}"));
    assert!(has_redaction_marker(&out), "Stripe key not redacted: {out:?}");
    assert!(!out.contains(&key));
}

#[test]
fn redacts_openai_api_key() {
    let r = default_redactor();
    let key = "sk-proj-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH";
    let out = r.redact_line(&format!("OPENAI_API_KEY={key}"));
    assert!(has_redaction_marker(&out), "OpenAI key not redacted: {out:?}");
    assert!(!out.contains(key));
}

#[test]
fn redacts_anthropic_api_key() {
    let r = default_redactor();
    let key = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz_0123456789-AAA";
    let out = r.redact_line(&format!("ANTHROPIC_API_KEY={key}"));
    assert!(has_redaction_marker(&out), "Anthropic key not redacted: {out:?}");
    assert!(!out.contains(key));
}

#[test]
fn redacts_github_classic_pat() {
    let r = default_redactor();
    let key = "ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789";
    let out = r.redact_line(&format!("auth: {key}"));
    assert!(has_redaction_marker(&out), "GitHub PAT not redacted: {out:?}");
    assert!(!out.contains(key));
}

#[test]
fn redacts_github_oauth_token() {
    let r = default_redactor();
    let key = "gho_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789";
    let out = r.redact_line(&format!("auth: {key}"));
    assert!(has_redaction_marker(&out), "GitHub OAuth token not redacted: {out:?}");
}

#[test]
fn redacts_jwt() {
    let r = default_redactor();
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let out = r.redact_line(&format!("Authorization: {jwt}"));
    assert!(has_redaction_marker(&out), "JWT not redacted: {out:?}");
    assert!(!out.contains(jwt));
}

#[test]
fn redacts_bearer_token() {
    let r = default_redactor();
    let line = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature123XYZ";
    let out = r.redact_line(line);
    assert!(has_redaction_marker(&out), "Bearer token not redacted: {out:?}");
}

#[test]
fn redacts_email_address() {
    let r = default_redactor();
    let out = r.redact_line("contact: alice@example.com");
    assert!(
        has_redaction_kind(&out, "email"),
        "email not redacted: {out:?} (configurable, but default-on per ARCH §10)",
    );
    assert!(!out.contains("alice@example.com"));
}

#[test]
fn redacts_ssh_private_key_path() {
    let r = default_redactor();
    let out = r.redact_line("Key file: /home/alice/.ssh/id_rsa");
    assert!(has_redaction_marker(&out), "SSH path not redacted: {out:?}");
}

#[test]
fn redacts_aws_credentials_path() {
    let r = default_redactor();
    let out = r.redact_line("Credentials: /Users/bob/.aws/credentials");
    assert!(has_redaction_marker(&out), "AWS path not redacted: {out:?}");
}

#[test]
fn redacts_ssh_path_in_tilde_home() {
    let r = default_redactor();
    let out = r.redact_line("see ~/.ssh/id_rsa for the key");
    assert!(
        has_redaction_kind(&out, "path-ssh"),
        "tilde-home SSH path not redacted: {out:?}",
    );
    assert!(!out.contains("~/.ssh/id_rsa"));
}

#[test]
fn redacts_aws_path_in_tilde_home() {
    let r = default_redactor();
    let out = r.redact_line("creds at ~/.aws/credentials");
    assert!(
        has_redaction_kind(&out, "path-aws"),
        "tilde-home AWS path not redacted: {out:?}",
    );
}

#[test]
fn redacts_ghr_refresh_token() {
    let r = default_redactor();
    let key = format!("ghr_{}", "a".repeat(40));
    let out = r.redact_line(&format!("refresh: {key}"));
    assert!(has_redaction_kind(&out, "github-refresh"), "ghr_ token not redacted: {out:?}");
    assert!(!out.contains(&key));
}

#[test]
fn rejects_openai_none_permissive_variant() {
    let r = default_redactor();
    let out = r.redact_line("token: sk-None-abcdefghijklmnopqrstuvwxyz0123456789");
    assert!(
        !has_redaction_kind(&out, "openai"),
        "non-canonical openai prefix must not match: {out:?}",
    );
}

#[test]
fn redacts_openai_svcacct_admin_prefixes() {
    let r = default_redactor();
    let svc = "sk-svcacct-abcdefghijklmnopqrstuvwxyz0123456789";
    let adm = "sk-admin-abcdefghijklmnopqrstuvwxyz0123456789";
    let out = r.redact_line(&format!("{svc} {adm}"));
    assert!(!out.contains(svc), "svcacct must be redacted: {out:?}");
    assert!(!out.contains(adm), "admin must be redacted: {out:?}");
}

#[test]
fn preserves_non_secret_alphanum_strings() {
    let r = default_redactor();
    let out = r.redact_line("commit: 1a2b3c4d5e6f7890abcdef1234567890");
    assert!(
        !has_redaction_marker(&out),
        "30-char hex must not match secret patterns: {out:?}"
    );
    assert!(out.contains("1a2b3c4d5e6f7890abcdef1234567890"));
}

#[test]
fn preserves_normal_prose() {
    let r = default_redactor();
    let line = "The quick brown fox jumps over the lazy dog 42 times.";
    let out = r.redact_line(line);
    assert_eq!(out, line, "prose without secrets must round-trip unchanged");
}

#[test]
fn redaction_is_idempotent() {
    let r = default_redactor();
    let once = r.redact_line("token=AKIAIOSFODNN7EXAMPLE alice@example.com");
    let twice = r.redact_line(&once);
    assert_eq!(once, twice, "redacting a redacted line must be a no-op");
}

#[test]
fn redacts_multiple_secrets_in_one_line() {
    let r = default_redactor();
    let line =
        "AKIAIOSFODNN7EXAMPLE and Bearer sk-proj-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH";
    let out = r.redact_line(line);
    assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(!out.contains("sk-proj-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH"));
}

#[test]
fn jsonl_line_remains_parseable_after_redaction() {
    let r = default_redactor();
    let line = r#"{"msg":"login as alice@example.com","token":"ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789"}"#;
    let out = r.redact_line(line);
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&out);
    assert!(parsed.is_ok(), "redacted JSONL line must remain parseable: {out:?}");
}

#[test]
fn jsonl_with_bearer_token_remains_parseable() {
    let r = default_redactor();
    let line = r#"{"hdr":"Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature123","next":"y"}"#;
    let out = r.redact_line(line);
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&out);
    assert!(parsed.is_ok(), "bearer redaction must not eat trailing JSON: {out:?}");
}

#[test]
fn clean_lines_skip_replace_loop() {
    let r = default_redactor();
    let mut lines = String::with_capacity(1024 * 1024);
    while lines.len() < 1024 * 1024 {
        lines.push_str(r#"{"role":"user","text":"the quick brown fox jumps"}\n"#);
    }
    for line in lines.lines() {
        let out = r.redact_line(line);
        assert_eq!(out, line, "clean line must round-trip byte-for-byte");
    }
}

#[test]
fn jsonl_with_ssh_path_remains_parseable() {
    let r = default_redactor();
    let line = r#"{"file":"/home/alice/.ssh/id_rsa","next":"y"}"#;
    let out = r.redact_line(line);
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&out);
    assert!(parsed.is_ok(), "ssh path redaction must not eat trailing JSON: {out:?}");
}

#[test]
fn redacts_ssn_us() {
    let r = default_redactor();
    let out = r.redact_line("SSN: 123-45-6789 on file");
    assert!(has_redaction_kind(&out, "ssn-us"), "SSN not redacted: {out:?}");
    assert!(!out.contains("123-45-6789"));
}

#[test]
fn ssn_us_preserves_zip_plus_four() {
    let r = default_redactor();
    let line = "Ship to ZIP 90210-1234";
    let out = r.redact_line(line);
    assert_eq!(out, line, "ZIP+4 must not match SSN pattern: {out:?}");
}

#[test]
fn redacts_phone_us_10d_dashed() {
    let r = default_redactor();
    let out = r.redact_line("Call (415) 555-0143 for service");
    assert!(has_redaction_kind(&out, "phone-us-10d"), "phone not redacted: {out:?}");
    assert!(!out.contains("555-0143"));
}

#[test]
fn phone_pattern_preserves_http_status_pairs() {
    let r = default_redactor();
    let input = "HTTP 404-5230 response from upstream";
    assert_eq!(r.redact_line(input), input, "HTTP status pairs must not redact");
}

#[test]
fn phone_pattern_preserves_short_alpha_strings() {
    let r = default_redactor();
    let line = "version=v1.2-rc3 commit=abc-defg";
    let out = r.redact_line(line);
    assert_eq!(out, line, "alpha-suffix tokens must not match phone pattern: {out:?}");
}

#[test]
fn phone_pattern_preserves_zip_plus_four() {
    let r = default_redactor();
    let line = "mailing address ZIP 10024-1234 New York";
    let out = r.redact_line(line);
    assert_eq!(out, line, "ZIP+4 must not match phone pattern: {out:?}");
}

#[test]
fn redacts_credit_card_dashed() {
    let r = default_redactor();
    let out = r.redact_line("CC: 4111-1111-1111-1111 (test)");
    assert!(has_redaction_kind(&out, "credit-card"), "CC not redacted: {out:?}");
    assert!(!out.contains("4111-1111-1111-1111"));
}

#[test]
fn credit_card_preserves_non_card_runs() {
    let r = default_redactor();
    let line = "build 1234-abcd-5678 not a card";
    let out = r.redact_line(line);
    assert!(
        !has_redaction_kind(&out, "credit-card"),
        "alphanum run must not look like a card: {out:?}",
    );
}

#[test]
fn credit_card_requires_separators() {
    let r = default_redactor();
    let line = "ts=1700000000000000 (16-digit timestamp)";
    let out = r.redact_line(line);
    assert!(
        !has_redaction_kind(&out, "credit-card"),
        "unseparated 16-digit run must not match credit-card: {out:?}",
    );
}

#[test]
fn redacts_iban_gb_form() {
    let r = default_redactor();
    let out = r.redact_line("IBAN GB29 NWBK 6016 1331 9268 19 noted");
    assert!(has_redaction_kind(&out, "iban"), "IBAN not redacted: {out:?}");
    assert!(!out.contains("9268 19"), "IBAN tail not redacted: {out:?}");
    assert!(!out.contains("GB29"), "IBAN head not redacted: {out:?}");
}

#[test]
fn iban_preserves_uppercase_words() {
    let r = default_redactor();
    let line = "USA UK CA DE FR IT NL";
    let out = r.redact_line(line);
    assert_eq!(out, line, "country code abbreviations must not match IBAN: {out:?}");
}

#[test]
fn iban_preserves_short_prefix_runs() {
    let r = default_redactor();
    let line = "code GB29 NWBK is too short to be an IBAN";
    let out = r.redact_line(line);
    assert!(!has_redaction_kind(&out, "iban"), "short prefix must not match IBAN: {out:?}");
}

#[test]
fn redacts_ipv4_in_log_line() {
    let r = default_redactor();
    let out = r.redact_line("client 203.0.113.42 connected");
    assert!(has_redaction_kind(&out, "ipv4"), "IPv4 not redacted: {out:?}");
    assert!(!out.contains("203.0.113.42"));
}

#[test]
fn ipv4_preserves_three_part_versions() {
    let r = default_redactor();
    let line = "stoa-cli v0.1.2 (build 9)";
    let out = r.redact_line(line);
    assert_eq!(out, line, "three-part version must not match IPv4: {out:?}");
}

#[test]
fn redacts_mac_address_colon_form() {
    let r = default_redactor();
    let out = r.redact_line("iface eth0 mac 00:1B:44:11:3A:B7");
    assert!(has_redaction_kind(&out, "mac-address"), "MAC not redacted: {out:?}");
}

#[test]
fn redacts_mac_address_dash_form_lowercase() {
    let r = default_redactor();
    let out = r.redact_line("hw addr: 00-1b-44-11-3a-b7");
    assert!(has_redaction_kind(&out, "mac-address"), "lowercase MAC not redacted: {out:?}");
}

#[test]
fn redacts_canary_token() {
    let r = default_redactor();
    let out = r.redact_line("ssn: CANARY_SSN_ZVR5XO4K (do not forward)");
    assert!(has_redaction_kind(&out, "canary-token"), "canary not redacted: {out:?}");
    assert!(!out.contains("CANARY_SSN_ZVR5XO4K"));
}

#[test]
fn canary_token_preserves_unrelated_uppercase() {
    let r = default_redactor();
    let line = "ALLCAPS_NORMAL_WORD";
    let out = r.redact_line(line);
    assert_eq!(out, line, "non-CANARY uppercase must not match: {out:?}");
}
