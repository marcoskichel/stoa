use std::path::Path;

use crate::{Hit, RecallError, SearchParams};

/// Swappable recall backend.
///
/// All three operations are required so `stoa index rebuild` can regenerate
/// the derived `.stoa/` layer from `raw/` + `wiki/` + `sessions/` alone.
pub trait RecallBackend: Send + Sync {
    /// Search for documents matching `params`.
    fn search(&self, params: &SearchParams) -> Result<Vec<Hit>, RecallError>;

    /// Add or update the index entry for a single file.
    fn index_path(&self, path: &Path) -> Result<(), RecallError>;

    /// Wipe and fully rebuild the index from the workspace on disk.
    fn rebuild(&self) -> Result<(), RecallError>;
}
