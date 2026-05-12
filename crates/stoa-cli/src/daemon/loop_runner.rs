//! Long-running daemon loop with graceful shutdown.
//!
//! Each worker is a `tokio::task` polling the queue via blocking
//! `stoa_capture::drain_once` (wrapped in `spawn_blocking`). The shutdown
//! token cancels on SIGINT / SIGTERM and the task tracker waits for every
//! worker to finish its current row before exiting.

use std::time::Duration;

use anyhow::Context;
use stoa_capture::WorkerConfig;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// Initial poll interval (matches the M3 research note).
const MIN_BACKOFF: Duration = Duration::from_millis(1);

/// Maximum poll interval; backoff resets to `MIN_BACKOFF` whenever a row
/// is found, so a busy queue never sleeps long.
const MAX_BACKOFF: Duration = Duration::from_millis(500);

/// Run the daemon loop blocking the current thread until SIGTERM/SIGINT.
pub(crate) fn serve(cfg: WorkerConfig, workers: usize) -> anyhow::Result<()> {
    let rt = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    rt.block_on(run_workers(cfg, workers))
}

async fn run_workers(cfg: WorkerConfig, workers: usize) -> anyhow::Result<()> {
    let cancel = CancellationToken::new();
    let tracker = TaskTracker::new();
    for _ in 0..workers {
        spawn_one(&tracker, &cancel, cfg.clone());
    }
    tracker.close();
    wait_for_shutdown(&cancel).await;
    tracker.wait().await;
    Ok(())
}

fn spawn_one(tracker: &TaskTracker, cancel: &CancellationToken, cfg: WorkerConfig) {
    let token = cancel.clone();
    tracker.spawn(async move { worker_loop(cfg, token).await });
}

async fn worker_loop(cfg: WorkerConfig, cancel: CancellationToken) {
    let mut backoff = MIN_BACKOFF;
    while !cancel.is_cancelled() {
        match tick_once(&cfg).await {
            Ok(true) => backoff = MIN_BACKOFF,
            _ => backoff = next_backoff(backoff),
        }
        if sleep_or_cancel(&cancel, backoff).await {
            return;
        }
    }
}

async fn tick_once(cfg: &WorkerConfig) -> anyhow::Result<bool> {
    let cfg = cfg.clone();
    let outcome = tokio::task::spawn_blocking(move || stoa_capture::drain_once(&cfg)).await??;
    Ok(outcome.is_some())
}

async fn sleep_or_cancel(cancel: &CancellationToken, dur: Duration) -> bool {
    tokio::select! {
        () = tokio::time::sleep(dur) => false,
        () = cancel.cancelled() => true,
    }
}

fn next_backoff(current: Duration) -> Duration {
    let next = current.saturating_mul(2);
    if next > MAX_BACKOFF {
        MAX_BACKOFF
    } else {
        next
    }
}

#[cfg(unix)]
async fn wait_for_shutdown(cancel: &CancellationToken) {
    use tokio::signal::unix::{SignalKind, signal};
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);
    let Ok(mut term) = signal(SignalKind::terminate()) else {
        let _ignored = ctrl_c.await;
        cancel.cancel();
        return;
    };
    tokio::select! {
        _ = &mut ctrl_c => {},
        _ = term.recv() => {},
    }
    cancel.cancel();
}

#[cfg(not(unix))]
async fn wait_for_shutdown(cancel: &CancellationToken) {
    let _ignored = tokio::signal::ctrl_c().await;
    cancel.cancel();
}
