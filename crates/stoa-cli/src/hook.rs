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
# session_id so duplicates are no-ops.

{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "stoa-hook --queue $STOA_WORKSPACE/.stoa/queue.db --session-id $CLAUDE_SESSION_ID --session-path $CLAUDE_SESSION_FILE --agent-id claude-code"
      }
    ],
    "SessionEnd": [
      {
        "type": "command",
        "command": "stoa-hook --queue $STOA_WORKSPACE/.stoa/queue.db --session-id $CLAUDE_SESSION_ID --session-path $CLAUDE_SESSION_FILE --agent-id claude-code"
      }
    ]
  }
}
"#;

/// Run `stoa hook install --platform <name>`.
pub(crate) fn install(platform: &str) -> anyhow::Result<()> {
    let snippet = snippet_for(platform)
        .with_context(|| format!("no built-in registration template for `{platform}`"))?;
    emit(snippet);
    Ok(())
}

fn snippet_for(platform: &str) -> anyhow::Result<&'static str> {
    match platform {
        "claude-code" => Ok(CLAUDE_CODE_SNIPPET),
        other => Err(anyhow!("unknown platform `{other}` — supported: claude-code")),
    }
}

#[expect(
    clippy::print_stdout,
    reason = "Snippet emission is the visible side-effect of the command."
)]
fn emit(snippet: &str) {
    print!("{snippet}");
}
