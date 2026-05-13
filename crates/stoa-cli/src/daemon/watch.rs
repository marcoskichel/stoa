//! Wiki file watcher (`notify-debouncer-full`).
//!
//! Watches `<workspace>/wiki` recursively. On a debounced `Modify` /
//! `Create` event for any `*.md` file, enqueues a `recall.request` row
//! with `method: "index_page"` so the recall worker re-tokenizes only
//! the changed page (vs. a full rebuild).
//!
//! Design notes:
//!
//! - Debounce 2 s — a 1 s window causes spurious double-fires from
//!   atomic-rename editors (vim, helix). Per architecture review.
//! - Returns a `Watcher` handle the caller drops on shutdown; the
//!   debouncer + its watch thread shut down on drop.
//! - `inotify` watch-limit (`ENOSPC`) is logged with a hint pointing to
//!   `/proc/sys/fs/inotify/max_user_watches`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, anyhow};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, NoCache, new_debouncer};
use stoa_queue::Queue;

const DEBOUNCE_WINDOW: Duration = Duration::from_secs(2);

/// Spawn a debounced watcher over `<workspace>/wiki`. Returns the handle
/// the caller MUST keep alive; dropping it stops the watch thread.
pub(crate) fn spawn_watcher(
    workspace_root: &Path,
    queue_path: &Path,
) -> anyhow::Result<Debouncer<notify::RecommendedWatcher, NoCache>> {
    let wiki_dir = workspace_root.join("wiki");
    if !wiki_dir.is_dir() {
        std::fs::create_dir_all(&wiki_dir)
            .with_context(|| format!("creating `{}`", wiki_dir.display()))?;
    }
    let queue_path_owned = queue_path.to_path_buf();
    let workspace_owned = workspace_root.to_path_buf();
    let mut debouncer = new_debouncer(
        DEBOUNCE_WINDOW,
        None,
        move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
            handle_event(&workspace_owned, &queue_path_owned, result);
        },
    )
    .context("starting wiki watcher")?;
    debouncer
        .watch(wiki_dir.as_path(), RecursiveMode::Recursive)
        .map_err(translate_watcher_error)?;
    Ok(debouncer)
}

fn handle_event(
    workspace_root: &Path,
    queue_path: &Path,
    result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>,
) {
    let events = match result {
        Ok(e) => e,
        Err(errors) => {
            for err in errors {
                tracing::warn!(error = %err, "wiki watcher error");
            }
            return;
        },
    };
    for ev in events {
        if !is_relevant(ev.event.kind) {
            continue;
        }
        for path in &ev.event.paths {
            if let Some(rel) = relative_md(workspace_root, path) {
                enqueue_index(queue_path, &rel);
            }
        }
    }
}

fn is_relevant(kind: EventKind) -> bool {
    matches!(kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
}

fn relative_md(workspace_root: &Path, abs: &Path) -> Option<String> {
    if abs.extension().is_none_or(|e| e != "md") {
        return None;
    }
    let rel = abs.strip_prefix(workspace_root).ok()?;
    Some(rel.to_string_lossy().into_owned())
}

fn enqueue_index(queue_path: &Path, rel_path: &str) {
    let payload = serde_json::json!({
        "method": "index_page",
        "args": {"path": rel_path},
    });
    let session_id = page_id_from_rel(rel_path);
    match Queue::open(queue_path) {
        Ok(q) => {
            if let Err(e) =
                q.insert_lane("recall.request", "wiki.page.written", &session_id, &payload)
            {
                tracing::warn!(error = %e, "wiki watcher enqueue failed");
            }
        },
        Err(e) => tracing::warn!(error = %e, "wiki watcher could not open queue"),
    }
}

fn page_id_from_rel(rel: &str) -> String {
    PathBuf::from(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_owned()
}

fn translate_watcher_error(err: notify::Error) -> anyhow::Error {
    if let notify::ErrorKind::Io(ref io) = err.kind
        && (io.kind() == std::io::ErrorKind::StorageFull
            || io.raw_os_error() == Some(libc_enospc()))
    {
        return anyhow!(
            "wiki watcher hit inotify limit; raise `/proc/sys/fs/inotify/max_user_watches` \
             (see kernel docs); underlying error: {err}",
        );
    }
    anyhow!(err)
}

#[cfg(target_os = "linux")]
const fn libc_enospc() -> i32 {
    28
}

#[cfg(not(target_os = "linux"))]
const fn libc_enospc() -> i32 {
    -1
}
