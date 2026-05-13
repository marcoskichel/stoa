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
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, anyhow};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache, new_debouncer};
use stoa_queue::Queue;

const DEBOUNCE_WINDOW: Duration = Duration::from_secs(2);

/// Spawn a debounced watcher over `<workspace>/wiki`. Returns the handle
/// the caller MUST keep alive; dropping it stops the watch thread.
pub(crate) fn spawn_watcher(
    workspace_root: &Path,
    queue_path: &Path,
) -> anyhow::Result<Debouncer<notify::RecommendedWatcher, RecommendedCache>> {
    let wiki_dir = workspace_root.join("wiki");
    if !wiki_dir.is_dir() {
        std::fs::create_dir_all(&wiki_dir)
            .with_context(|| format!("creating `{}`", wiki_dir.display()))?;
    }
    let queue = Arc::new(
        Queue::open(queue_path)
            .with_context(|| format!("opening queue `{}`", queue_path.display()))?,
    );
    let workspace_owned = workspace_root.to_path_buf();
    let mut debouncer = new_debouncer(
        DEBOUNCE_WINDOW,
        None,
        move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
            catch_unwind_handle_event(&workspace_owned, &queue, result);
        },
    )
    .context("starting wiki watcher")?;
    debouncer
        .watch(wiki_dir.as_path(), RecursiveMode::Recursive)
        .map_err(translate_watcher_error)?;
    Ok(debouncer)
}

/// Wrap `handle_event` in `catch_unwind` so a panic inside the
/// notify-thread closure is logged + swallowed instead of unwinding
/// across the FFI boundary into `notify-debouncer-full` (UB).
fn catch_unwind_handle_event(
    workspace_root: &Path,
    queue: &Arc<Queue>,
    result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>,
) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_event(workspace_root, queue, result);
    }));
    if let Err(payload) = outcome {
        let msg = panic_message(&payload);
        tracing::error!(panic = %msg, "wiki watcher callback panicked; event dropped");
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_owned();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "(non-string panic payload)".to_owned()
}

fn handle_event(
    workspace_root: &Path,
    queue: &Arc<Queue>,
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
        let Some(method) = method_for_event(ev.event.kind) else {
            continue;
        };
        for path in &ev.event.paths {
            if let Some(rel) = relative_md(workspace_root, path) {
                enqueue(queue, method, &rel);
            }
        }
    }
}

/// Map a notify event kind to the recall method we want the daemon to
/// run, or `None` to drop the event entirely.
fn method_for_event(kind: EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) | EventKind::Modify(_) => Some("index_page"),
        EventKind::Remove(_) => Some("remove_page"),
        _ => None,
    }
}


fn relative_md(workspace_root: &Path, abs: &Path) -> Option<String> {
    if abs.extension().is_none_or(|e| e != "md") {
        return None;
    }
    if is_symlink_or_under_symlink(abs) {
        tracing::warn!(path = %abs.display(), "wiki watcher refusing symlinked path");
        return None;
    }
    let rel = abs.strip_prefix(workspace_root).ok()?;
    Some(rel.to_string_lossy().into_owned())
}

/// Refuse the path if it (or any parent component up to the workspace
/// root) is a symlink. Combined with the workspace-root canonicalization
/// in `recall_drain`, this prevents a hostile `wiki/link.md ->
/// /etc/shadow` from becoming searchable.
///
/// The check uses `symlink_metadata` so we do NOT traverse the link;
/// `Path::is_file` would silently follow it.
fn is_symlink_or_under_symlink(abs: &Path) -> bool {
    if let Ok(meta) = std::fs::symlink_metadata(abs)
        && meta.file_type().is_symlink()
    {
        return true;
    }
    let mut cursor = abs.parent();
    while let Some(p) = cursor {
        match std::fs::symlink_metadata(p) {
            Ok(m) if m.file_type().is_symlink() => return true,
            Ok(_) => {},
            Err(_) => return false,
        }
        cursor = p.parent();
    }
    false
}

fn enqueue(queue: &Arc<Queue>, method: &str, rel_path: &str) {
    let payload = serde_json::json!({
        "method": method,
        "args": {"path": rel_path, "page_id": page_id_from_rel(rel_path)},
    });
    let event = match method {
        "remove_page" => "wiki.page.removed",
        _ => "wiki.page.written",
    };
    let session_id = format!("{method}:{}", page_id_from_rel(rel_path));
    if let Err(e) = queue.insert_lane("recall.request", event, &session_id, &payload) {
        tracing::warn!(error = %e, "wiki watcher enqueue failed");
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
