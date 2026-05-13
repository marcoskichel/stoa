//! Stoa capture hook (Claude Code `Stop` / `SessionEnd` integration).
//!
//! Reads the Claude Code hook payload on stdin, pulls the
//! `transcript_path`, sends a single `mine` RPC to `stoa-recalld` over
//! its Unix socket, and exits. The daemon is responsible for actually
//! ingesting the transcript; the hook is fire-and-forget.
//!
//! Cold-start budget: <10 ms p95 when the daemon is up. The socket
//! write is best-effort — if the daemon is down or the socket is
//! missing, the hook exits 0 (silent) so a missing daemon never
//! breaks an agent session.

#![doc(html_no_source)]

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

/// Max stdin bytes — Claude Code payloads are <2 KiB in practice.
const MAX_PAYLOAD_BYTES: u64 = 256 * 1024;

/// Mine RPC deadline. Capture is fire-and-forget; we just want to know
/// the daemon accepted the request, then exit.
const RPC_DEADLINE: Duration = Duration::from_millis(500);

#[derive(Debug, Default, Deserialize)]
struct HookPayload {
    #[serde(default)]
    transcript_path: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    hook_event_name: Option<String>,
}

fn main() -> ExitCode {
    let payload = read_payload(std::io::stdin().lock());
    let Some(transcript_path) = payload.transcript_path.filter(|s| !s.is_empty()) else {
        return ExitCode::SUCCESS;
    };
    let socket = default_socket_path();
    drop(fire_mine(&socket, &transcript_path, payload.session_id.as_deref()));
    drop(payload.hook_event_name);
    ExitCode::SUCCESS
}

fn read_payload<R: Read>(mut stdin: R) -> HookPayload {
    let mut buf = Vec::with_capacity(2048);
    let mut limited = (&mut stdin).take(MAX_PAYLOAD_BYTES + 1);
    if limited.read_to_end(&mut buf).is_err() {
        return HookPayload::default();
    }
    if buf.is_empty() || buf.len() as u64 > MAX_PAYLOAD_BYTES {
        return HookPayload::default();
    }
    serde_json::from_slice(&buf).unwrap_or_default()
}

fn default_socket_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("STOA_RECALLD_SOCKET") {
        return PathBuf::from(explicit);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("stoa-recalld.sock");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_owned());
    PathBuf::from(format!("/tmp/stoa-recalld-{user}.sock"))
}

fn fire_mine(
    socket: &Path,
    transcript_path: &str,
    session_id: Option<&str>,
) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_write_timeout(Some(RPC_DEADLINE))?;
    stream.set_read_timeout(Some(RPC_DEADLINE))?;
    let mut params = serde_json::Map::new();
    let _prev_src = params.insert("source_file".to_owned(), json!(transcript_path));
    if let Some(id) = session_id {
        let _prev_sid = params.insert("session_id".to_owned(), json!(id));
    }
    let envelope = json!({
        "method": "mine",
        "params": serde_json::Value::Object(params),
    });
    let line = serde_json::to_vec(&envelope)?;
    stream.write_all(&line)?;
    stream.write_all(b"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = Vec::with_capacity(128);
    drop(stream.read_to_end(&mut response));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HookPayload, default_socket_path, read_payload};

    #[test]
    fn empty_stdin_is_default() {
        let payload = read_payload(std::io::empty());
        assert!(payload.transcript_path.is_none());
    }

    #[test]
    fn parses_minimal_payload() {
        let json = br#"{"transcript_path":"/tmp/t.jsonl","session_id":"01J"}"#;
        let p: HookPayload = serde_json::from_slice(json).unwrap_or_default();
        assert_eq!(p.transcript_path.as_deref(), Some("/tmp/t.jsonl"));
    }

    #[test]
    fn socket_path_resolution_is_deterministic() {
        // NOTE: without env overrides, the path must end with `stoa-recalld.sock`.
        let p = default_socket_path();
        assert!(
            p.to_string_lossy().ends_with("stoa-recalld.sock"),
            "unexpected default path: {}",
            p.display()
        );
    }
}
