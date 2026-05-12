//! File-walker entry point. Scans every `.rs` source under the given roots
//! and delegates per-source analysis to [`comments::check_source`].

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

mod comments;

pub(crate) use self::comments::Finding;

/// Recursively scan `roots` and return any forbidden-comment findings.
///
/// File arguments bypass the directory-exclusion filter; only `WalkDir`
/// traversals honor `is_excluded`. This lets callers pass a single fixture
/// file under `tests/fixtures/` without that fixture being silently skipped.
pub(crate) fn run(roots: &[PathBuf]) -> Vec<Finding> {
    let mut out = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_rust_source(root) {
                out.extend(check_file(root));
            }
            continue;
        }
        for entry in WalkDir::new(root).into_iter().flatten() {
            let path = entry.path();
            if is_rust_source(path) && !is_excluded(path) {
                out.extend(check_file(path));
            }
        }
    }
    out
}

fn is_rust_source(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "rs")
}

fn is_excluded(path: &Path) -> bool {
    path.components()
        .any(|comp| matches!(comp.as_os_str().to_str(), Some("target" | "fixtures" | ".git")))
}

fn check_file(path: &Path) -> Vec<Finding> {
    let Ok(src) = fs::read_to_string(path) else {
        return Vec::new();
    };
    comments::check_source(path, &src)
}
