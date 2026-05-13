//! `stoa daemon start|stop|status` — manage the `stoa-recalld` lifecycle.
//!
//! `start` launches `stoa-recalld --foreground` via `setsid nohup` so
//! the daemon detaches from the controlling terminal. `stop` reads the
//! PID file and sends SIGTERM. `status` is the only async path —
//! it queries the daemon over its Unix socket and renders the health
//! response.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use stoa_recall::{MempalaceBackend, RecallBackend, default_socket_path};

/// Spawn the daemon in the background.
pub(crate) fn start() -> Result<()> {
    let paths = DaemonPaths::resolve();
    paths.ensure_dirs();
    spawn_daemon(&paths)?;
    println(&format!(
        "Launched stoa-recalld; socket={} pid={} log={}",
        paths.socket.display(),
        paths.pid_file.display(),
        paths.log_file.display(),
    ));
    println("Wait a few seconds, then run `stoa daemon status` to confirm.");
    Ok(())
}

struct DaemonPaths {
    socket: PathBuf,
    pid_file: PathBuf,
    log_file: PathBuf,
}

impl DaemonPaths {
    fn resolve() -> Self {
        Self {
            socket: default_socket_path(),
            pid_file: pid_file_path(),
            log_file: log_file_path(),
        }
    }

    fn ensure_dirs(&self) {
        for p in [&self.socket, &self.pid_file, &self.log_file] {
            if let Some(parent) = p.parent() {
                let _ignored = std::fs::create_dir_all(parent);
            }
        }
    }
}

fn spawn_daemon(paths: &DaemonPaths) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "setsid nohup {bin} --foreground --socket {sock} --pid-file {pid} >>{log} 2>&1 &",
            bin = shell_quote(&daemon_binary()),
            sock = shell_quote(&paths.socket.to_string_lossy()),
            pid = shell_quote(&paths.pid_file.to_string_lossy()),
            log = shell_quote(&paths.log_file.to_string_lossy()),
        ))
        .status()
        .context("invoking `setsid nohup stoa-recalld`")?;
    if !status.success() {
        bail!("spawn failed (exit {status:?})");
    }
    Ok(())
}

/// Stop the daemon via SIGTERM.
pub(crate) fn stop() -> Result<()> {
    let pid_file = pid_file_path();
    let raw = std::fs::read_to_string(&pid_file)
        .with_context(|| format!("reading {}", pid_file.display()))?;
    let pid: i32 = raw.trim().parse().context("parsing pid file")?;
    let status = Command::new("kill")
        .arg(pid.to_string())
        .status()
        .context("kill")?;
    if !status.success() {
        bail!("kill {pid} failed");
    }
    let _removed = std::fs::remove_file(&pid_file);
    println(&format!("Sent SIGTERM to pid {pid}"));
    Ok(())
}

/// Health-probe the daemon over its Unix socket.
pub(crate) async fn status() -> Result<()> {
    let backend = MempalaceBackend::from_env();
    let resp = backend
        .health()
        .await
        .map_err(|e| anyhow!("daemon health probe failed: {e}"))?;
    println(&format!(
        "Daemon healthy.\n{}",
        serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.to_string()),
    ));
    Ok(())
}

fn daemon_binary() -> String {
    std::env::var("STOA_RECALLD_BIN").unwrap_or_else(|_| "stoa-recalld".to_owned())
}

fn pid_file_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("STOA_RECALLD_PID_FILE") {
        return PathBuf::from(explicit);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("stoa-recalld.pid");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_owned());
    PathBuf::from(format!("/tmp/stoa-recalld-{user}.pid"))
}

fn log_file_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("STOA_RECALLD_LOG_FILE") {
        return PathBuf::from(explicit);
    }
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(state_home).join("stoa").join("recalld.log");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/state/stoa/recalld.log");
    }
    PathBuf::from("/tmp/stoa-recalld.log")
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', r"'\''");
    format!("'{escaped}'")
}

#[expect(clippy::print_stdout, reason = "User-facing CLI status output.")]
fn println(msg: &str) {
    println!("{msg}");
}
