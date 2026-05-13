//! `stoa daemon` — long-running capture worker(s) + `--once` drain mode.
//!
//! See [ARCHITECTURE.md §7]. Worker layout:
//!
//! - `--once`: open the queue, drain a single row if available, exit. No
//!   async runtime.
//! - default: spawn N async workers polling the queue with exponential
//!   backoff (1ms → 500ms). Ctrl-C / SIGTERM cancels all workers and
//!   waits for them to finish their current row.

mod loop_runner;

use std::path::Path;

use anyhow::Context;
use stoa_capture::WorkerConfig;

use crate::workspace::Workspace;

/// Default worker count for the long-running daemon.
const DEFAULT_WORKERS: usize = 4;

/// Entry point dispatched from `stoa daemon [--once]`.
pub(crate) fn run(once: bool) -> anyhow::Result<()> {
    let ws = Workspace::current().context("locating Stoa workspace")?;
    let cfg = worker_config(&ws);
    ensure_dirs(&cfg)?;
    if once {
        let _ignored = stoa_capture::drain_once(&cfg).context("draining capture queue")?;
        Ok(())
    } else {
        loop_runner::serve(cfg, DEFAULT_WORKERS)
    }
}

fn worker_config(ws: &Workspace) -> WorkerConfig {
    WorkerConfig {
        queue_path: ws.root.join(".stoa/queue.db"),
        sessions_dir: ws.root.join("sessions"),
        audit_log: ws.root.join(".stoa/audit.log"),
        workspace_root: ws.root.clone(),
    }
}

fn ensure_dirs(cfg: &WorkerConfig) -> anyhow::Result<()> {
    create_parent(&cfg.queue_path)?;
    std::fs::create_dir_all(&cfg.sessions_dir)
        .with_context(|| format!("creating `{}`", cfg.sessions_dir.display()))?;
    create_parent(&cfg.audit_log)?;
    Ok(())
}

fn create_parent(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating `{}`", parent.display()))?;
    }
    Ok(())
}
