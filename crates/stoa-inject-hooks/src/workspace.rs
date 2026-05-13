//! Workspace lookup for the inject hook.
//!
//! Walks up from `cwd` looking for `STOA.md`. Returns `None` (not an
//! error) if no marker is found — the hook degrades to "no injection"
//! per ARCH §6.2 rather than blocking session start.

use std::path::{Path, PathBuf};

const STOA_MD: &str = "STOA.md";

/// Resolved workspace paths needed by the inject hook.
#[derive(Debug, Clone)]
pub(crate) struct InjectWorkspace {
    pub(crate) root: PathBuf,
}

impl InjectWorkspace {
    /// Path of the workspace's `.stoa/recall.db` (may not exist).
    pub(crate) fn recall_db(&self) -> PathBuf {
        self.root.join(".stoa").join("recall.db")
    }

    /// Path of the workspace's `.stoa/audit.log` (created on first write).
    pub(crate) fn audit_log(&self) -> PathBuf {
        self.root.join(".stoa").join("audit.log")
    }

    /// Path of the workspace's `wiki/` dir.
    pub(crate) fn wiki(&self) -> PathBuf {
        self.root.join("wiki")
    }
}

/// Walk up from `start` until a directory containing `STOA.md` is found.
///
/// Returns `None` if no marker is found before the filesystem root —
/// callers treat that as "no workspace, emit empty injection".
pub(crate) fn find_workspace(start: &Path) -> Option<InjectWorkspace> {
    let mut here: Option<&Path> = Some(start);
    while let Some(dir) = here {
        if dir.join(STOA_MD).is_file() {
            return Some(InjectWorkspace { root: dir.to_path_buf() });
        }
        here = dir.parent();
    }
    None
}
