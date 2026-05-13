//! Claude Code hook payload parsing.
//!
//! Same binary handles two Claude Code hook events:
//!
//! - `SessionStart` — fires once per session boot. Payload carries
//!   `session_id`, `cwd`, `source` (startup/resume/clear/compact).
//! - `UserPromptSubmit` — fires before each user message. Payload
//!   carries `session_id`, `cwd`, `prompt` (the user's submitted text).
//!
//! Both events are coalesced into one [`HookPayload`] with optional
//! fields. Missing fields degrade to "no injection" (the hook returns
//! an empty `additionalContext`) rather than producing an error.

use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

/// Hook event flavor — drives the `additionalContext` JSON output tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    /// `SessionStart` — fires on session boot. Query is built from
    /// cwd + git remote + recently-edited wiki pages.
    SessionStart,
    /// `UserPromptSubmit` — fires per user prompt. Query is built
    /// primarily from the prompt text plus the workspace signals.
    UserPromptSubmit,
}

impl HookEvent {
    /// Wire name for the `hookEventName` field in the response JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
        }
    }

    fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("UserPromptSubmit") => Self::UserPromptSubmit,
            _ => Self::SessionStart,
        }
    }
}

/// Parsed Claude Code hook payload.
///
/// All fields optional. The hook never panics on a malformed payload —
/// missing values produce an empty injection.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct HookPayload {
    #[serde(default)]
    pub(crate) hook_event_name: Option<String>,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
    #[serde(default)]
    pub(crate) cwd: Option<PathBuf>,
    #[serde(default)]
    pub(crate) prompt: Option<String>,
}

impl HookPayload {
    /// Borrow the session id with `""` as the default.
    pub(crate) fn session_id_str(&self) -> &str {
        self.session_id.as_deref().unwrap_or("")
    }

    /// Return the resolved hook event flavor.
    pub(crate) fn event(&self) -> HookEvent {
        HookEvent::parse(self.hook_event_name.as_deref())
    }

    /// Wire name for the response `hookEventName`.
    pub(crate) fn hook_event_name_str(&self) -> &str {
        self.event().as_str()
    }
}

/// Cap on stdin payload bytes. Real Claude Code payloads are <8 KiB.
const MAX_PAYLOAD_BYTES: u64 = 256 * 1024;

/// Read up to [`MAX_PAYLOAD_BYTES`] of stdin and parse as a hook payload.
///
/// Empty or oversize input both degrade to the default payload so the
/// hook stays a no-op rather than failing the agent's startup path.
pub(crate) fn read_payload<R: Read>(stdin: R) -> Result<HookPayload> {
    let mut limited = stdin.take(MAX_PAYLOAD_BYTES + 1);
    let mut buf = String::new();
    let bytes = limited.read_to_string(&mut buf)?;
    if bytes as u64 > MAX_PAYLOAD_BYTES || buf.trim().is_empty() {
        return Ok(HookPayload::default());
    }
    Ok(serde_json::from_str(&buf).unwrap_or_default())
}
