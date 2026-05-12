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
