//! E2E quality gate: `stoa hook install --platform claude-code`.
//!
//! Spec source: [ROADMAP.md M3] + [ARCHITECTURE.md §7].
//!
//! v0.1 deliberately prints the registration snippet rather than mutating
//! the user's Claude Code config (per M3 daemon research note: auto-install
//! requires privilege escalation and is fragile across distros).

mod common;

use common::{init, stderr, stoa, workspace};

#[test]
fn hook_install_emits_claude_code_registration() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["hook", "install", "--platform", "claude-code"]);
    assert!(
        out.status.success(),
        "`stoa hook install --platform claude-code` must succeed: {}",
        stderr(&out),
    );
    let text = common::stdout(&out);
    assert!(
        text.contains("Stop") || text.contains("SessionEnd"),
        "output must reference the Claude Code Stop/SessionEnd hook: {text:?}",
    );
    assert!(
        text.contains("stoa-hook") || text.contains("stoa hook"),
        "output must reference the stoa-hook binary: {text:?}",
    );
}

#[test]
fn hook_install_unknown_platform_exits_non_zero() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["hook", "install", "--platform", "not-real"]);
    assert!(!out.status.success(), "unknown platform must error rather than silently noop");
}
