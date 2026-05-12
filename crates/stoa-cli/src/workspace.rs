//! Workspace detection + filesystem layout helpers.
//!
//! A "stoa workspace" is a directory containing `STOA.md` at its root. The
//! CLI walks up from the current working directory to find one; `init` is
//! the only command that creates one from scratch.

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};

/// Marker file. Presence at a directory root means "this is a stoa workspace".
pub(crate) const STOA_MD: &str = "STOA.md";

/// Filesystem layout of a Stoa workspace (ARCHITECTURE §1).
#[derive(Debug, Clone)]
pub(crate) struct Workspace {
    /// Workspace root — the directory containing `STOA.md`.
    pub(crate) root: PathBuf,
}

impl Workspace {
    /// Locate the workspace by walking up from `cwd`. Returns an error
    /// mentioning "not in a Stoa workspace" if no marker is found.
    pub(crate) fn find_from(cwd: &Path) -> anyhow::Result<Self> {
        let mut here = Some(cwd);
        while let Some(dir) = here {
            if dir.join(STOA_MD).is_file() {
                return Ok(Self { root: dir.to_path_buf() });
            }
            here = dir.parent();
        }
        Err(anyhow!("not in a Stoa workspace (no `{STOA_MD}` found from {})", cwd.display(),))
    }

    /// Locate the workspace from the current process cwd.
    pub(crate) fn current() -> anyhow::Result<Self> {
        let cwd = std::env::current_dir().context("reading current dir")?;
        Self::find_from(&cwd)
    }

    /// `<root>/wiki`
    pub(crate) fn wiki(&self) -> PathBuf {
        self.root.join("wiki")
    }

    /// `<root>/wiki/index.md`
    pub(crate) fn index_md(&self) -> PathBuf {
        self.wiki().join("index.md")
    }

    /// `<root>/wiki/log.md`
    pub(crate) fn log_md(&self) -> PathBuf {
        self.wiki().join("log.md")
    }

    /// `<root>/STOA.md`
    pub(crate) fn stoa_md(&self) -> PathBuf {
        self.root.join(STOA_MD)
    }

    /// `<root>/wiki/<subdir>`
    pub(crate) fn wiki_subdir(&self, dir: stoa_core::PageDir) -> PathBuf {
        self.wiki().join(dir.as_subdir())
    }

    /// Resolve the on-disk path for a page id (e.g. `ent-redis` →
    /// `wiki/entities/ent-redis.md`). Returns an error if the prefix is
    /// unknown.
    pub(crate) fn page_path(&self, id: &str) -> anyhow::Result<PathBuf> {
        let parsed = stoa_core::Id::parse(id)
            .ok_or_else(|| anyhow!("unknown id prefix in `{id}` (expected ent-/con-/syn-)"))?;
        Ok(self.wiki_subdir(parsed.dir).join(format!("{id}.md")))
    }
}
