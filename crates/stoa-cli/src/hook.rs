//! `stoa hook install --platform <name>` — emit a registration snippet
//! the user pastes into their agent platform's settings.
//!
//! v0.1 deliberately *prints* the snippet rather than mutating user
//! config (per the M3 daemon research note: auto-installation requires
//! privilege escalation and is fragile across distros).

use anyhow::{Context, anyhow};

/// Snippet emitted by `stoa hook install --platform claude-code`.
///
/// Claude Code recognizes two relevant lifecycle hooks (`Stop` and
/// `SessionEnd`); registering both gives us belt-and-braces coverage of
/// every clean exit path.
///
/// NOTE: every `$VAR` expansion is wrapped in double quotes so paths with
/// spaces or shell metacharacters (a workspace under `~/My Documents/...`,
/// a session id containing `;`) don't break the hook command. Claude Code
/// runs each entry through `/bin/sh -c`, so unquoted expansions would word-
/// split on whitespace. Operators copying this snippet inherit the
/// quoting and stay safe by default.
const CLAUDE_CODE_SNIPPET: &str = r#"# Claude Code hook registration for Stoa
#
# Paste the JSON below into your `~/.config/claude-code/settings.json`
# under the `hooks` block. The `stoa-hook` binary must be on $PATH; see
# `cargo install --path crates/stoa-hooks` from the Stoa repo.
#
# Stop      — fires on any clean end of an assistant response.
# SessionEnd — fires when the user ends the Claude Code session.
#
# Both are wired to the same handler; the worker is idempotent on
# session_id so duplicates are no-ops. Variable expansions are quoted to
# tolerate paths with spaces or shell metacharacters.

{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "stoa-hook --queue \"$STOA_WORKSPACE/.stoa/queue.db\" --session-id \"$CLAUDE_SESSION_ID\" --session-path \"$CLAUDE_SESSION_FILE\" --agent-id claude-code"
      }
    ],
    "SessionEnd": [
      {
        "type": "command",
        "command": "stoa-hook --queue \"$STOA_WORKSPACE/.stoa/queue.db\" --session-id \"$CLAUDE_SESSION_ID\" --session-path \"$CLAUDE_SESSION_FILE\" --agent-id claude-code"
      }
    ]
  }
}
"#;

/// SessionStart injection snippet (Claude Code).
///
/// `stoa-inject-hook` reads the JSON payload Claude Code sends on
/// stdin and writes a `hookSpecificOutput.additionalContext` JSON
/// document on stdout. The matcher pins it to fresh sessions
/// (`startup`); resume / clear / compact get their own hook points
/// in v0.2 per ARCHITECTURE.md §6.2.
const CLAUDE_CODE_INJECT_SESSION_START_SNIPPET: &str = r#"# Claude Code SessionStart injection hook for Stoa
#
# Paste the JSON below into your `~/.config/claude-code/settings.json`
# under the `hooks` block. The `stoa-inject-hook` binary must be on
# $PATH; see `cargo install --path crates/stoa-inject-hooks` from the
# Stoa repo. Default 10 s timeout; the hook degrades to "no injection"
# if recall is unhealthy or the workspace is missing.

{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "stoa-inject-hook",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
"#;

/// Run `stoa hook install --platform <name> [--inject <kind>]`.
pub(crate) fn install(platform: &str, inject: Option<&str>) -> anyhow::Result<()> {
    let snippet = snippet_for(platform, inject).with_context(|| {
        format!("no built-in registration template for `{platform}` (inject={inject:?})")
    })?;
    emit(snippet);
    Ok(())
}

fn snippet_for(platform: &str, inject: Option<&str>) -> anyhow::Result<&'static str> {
    match (platform, inject) {
        ("claude-code", None) => Ok(CLAUDE_CODE_SNIPPET),
        ("claude-code", Some("session-start")) => Ok(CLAUDE_CODE_INJECT_SESSION_START_SNIPPET),
        ("claude-code", Some(other)) => Err(anyhow!(
            "unknown --inject kind `{other}` for claude-code — supported: session-start"
        )),
        (other, _) => Err(anyhow!("unknown platform `{other}` — supported: claude-code")),
    }
}

#[expect(
    clippy::print_stdout,
    reason = "Snippet emission is the visible side-effect of the command."
)]
fn emit(snippet: &str) {
    print!("{snippet}");
}
