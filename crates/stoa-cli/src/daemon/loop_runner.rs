//! Long-running daemon loop with graceful shutdown.
//!
//! Each worker is a `tokio::task` polling the queue via blocking
//! `stoa_capture::drain_once` (wrapped in `spawn_blocking`). The shutdown
//! token cancels on SIGINT / SIGTERM and the task tracker waits for every
//! worker to finish its current row before exiting.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use stoa_capture::WorkerConfig;
use stoa_queue::Queue;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// Initial poll interval (matches the M3 research note).
const MIN_BACKOFF: Duration = Duration::from_millis(1);

/// Maximum poll interval; backoff resets to `MIN_BACKOFF` whenever a row
/// is found, so a busy queue never sleeps long.
const MAX_BACKOFF: Duration = Duration::from_millis(500);

/// Consecutive idle cycles at `MAX_BACKOFF` before the worker runs
/// `PRAGMA wal_checkpoint(TRUNCATE)`. Tuned so a quiet queue truncates
/// the WAL roughly every minute (~120 cycles × 500ms) without thrashing
/// during bursty traffic.
const IDLE_CYCLES_PER_CHECKPOINT: u32 = 120;

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
        spawn_one(&tracker, &cancel, cfg.clone())?;
    }
    tracker.close();
    wait_for_shutdown(&cancel).await;
    tracker.wait().await;
    Ok(())
}

fn spawn_one(
    tracker: &TaskTracker,
    cancel: &CancellationToken,
    cfg: WorkerConfig,
) -> anyhow::Result<()> {
    let token = cancel.clone();
    let queue = Arc::new(Queue::init(&cfg.queue_path).context("opening per-worker queue")?);
    tracker.spawn(async move { worker_loop(queue, cfg, token).await });
    Ok(())
}

async fn worker_loop(queue: Arc<Queue>, cfg: WorkerConfig, cancel: CancellationToken) {
    let mut backoff = MIN_BACKOFF;
    let mut idle_at_max: u32 = 0;
    while !cancel.is_cancelled() {
        if let Ok(true) = tick_once(&queue, &cfg).await {
            backoff = MIN_BACKOFF;
            idle_at_max = 0;
        } else {
            backoff = next_backoff(backoff);
            idle_at_max = maybe_checkpoint(&queue, backoff, idle_at_max).await;
        }
        if sleep_or_cancel(&cancel, backoff).await {
            return;
        }
    }
}

async fn tick_once(queue: &Arc<Queue>, cfg: &WorkerConfig) -> anyhow::Result<bool> {
    let q = Arc::clone(queue);
    let cfg = cfg.clone();
    let outcome =
        tokio::task::spawn_blocking(move || stoa_capture::drain_once_with(&q, &cfg)).await??;
    Ok(outcome.is_some())
}

/// Increment the idle-at-max counter; once it crosses
/// [`IDLE_CYCLES_PER_CHECKPOINT`], run a WAL truncate checkpoint and reset
/// the counter. Failures are swallowed (best-effort) — a missed
/// checkpoint just means the WAL keeps growing for one more cycle.
async fn maybe_checkpoint(queue: &Arc<Queue>, backoff: Duration, idle: u32) -> u32 {
    if backoff < MAX_BACKOFF {
        return 0;
    }
    let next = idle.saturating_add(1);
    if next < IDLE_CYCLES_PER_CHECKPOINT {
        return next;
    }
    let q = Arc::clone(queue);
    let _ignored =
        tokio::task::spawn_blocking(move || q.checkpoint().map_err(anyhow::Error::from)).await;
    0
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
