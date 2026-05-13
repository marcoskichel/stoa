//! `SessionStart` hook payload — typed view over Claude Code's stdin JSON.
//!
//! Every field is optional: Claude Code may add or rename fields between
//! versions, and missing values must degrade to "no injection" rather
//! than a hook failure (graceful degradation per ARCH §6.2).

use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

/// Parsed Claude-Code `SessionStart` payload.
///
/// All fields are optional. Defaults match the empty-string / `None`
/// expected by the audit + wrap layers, so a malformed payload never
/// short-circuits with a panic — it produces an empty injection
/// instead.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct SessionStartPayload {
    #[serde(default)]
    pub(crate) hook_event_name: Option<String>,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
    #[serde(default)]
    pub(crate) cwd: Option<PathBuf>,
}

impl SessionStartPayload {
    /// Borrow the session id with `""` as the default — the audit
    /// log records an empty string so a missing id stays grep-friendly.
    pub(crate) fn session_id_str(&self) -> &str {
        self.session_id.as_deref().unwrap_or("")
    }

    /// Return the literal `hook_event_name` or fall back to
    /// `"SessionStart"` so the audit log row stays consistent with
    /// the contract Claude Code pins on the inbound event.
    pub(crate) fn hook_event_name_str(&self) -> &str {
        self.hook_event_name.as_deref().unwrap_or("SessionStart")
    }
}

/// Read every byte of stdin and parse it as a `SessionStart` payload.
///
/// Empty stdin yields the default payload (no fields set) so invoking
/// the hook with no input mirrors invoking it from a workspace that has
/// no recall db: graceful no-op, success exit.
pub(crate) fn read_payload<R: Read>(mut stdin: R) -> Result<SessionStartPayload> {
    let mut buf = String::new();
    let _bytes = stdin.read_to_string(&mut buf)?;
    if buf.trim().is_empty() {
        return Ok(SessionStartPayload::default());
    }
    Ok(serde_json::from_str(&buf)?)
}
