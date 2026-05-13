//! Workspace lookup for the inject hook.
//!
//! Walks up from `cwd` looking for `STOA.md`. Returns `None` (not an
//! error) when no marker is found — the hook degrades to "no injection"
//! rather than blocking the session start or prompt submission path.

use std::path::{Path, PathBuf};

const STOA_MD: &str = "STOA.md";

/// Resolved workspace paths needed by the inject hook.
#[derive(Debug, Clone)]
pub(crate) struct InjectWorkspace {
    pub(crate) root: PathBuf,
}

impl InjectWorkspace {
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
