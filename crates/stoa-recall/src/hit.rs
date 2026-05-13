//! `Hit` — one ranked retrieval result returned by [`RecallBackend::search`].

use serde::{Deserialize, Serialize};

/// Open-ended metadata bag — passed through from `MemPalace` verbatim.
pub type Metadata = serde_json::Map<String, serde_json::Value>;

/// Stable doc id (a wiki page id, drawer id, or `session/<id>` ref).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocId(pub String);

impl DocId {
    /// Build a `DocId` from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DocId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DocId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Workspace-relative path the user can open.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourcePath(pub String);

impl SourcePath {
    /// Build a `SourcePath` from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SourcePath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SourcePath {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// One ranked retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    /// Stable id.
    pub doc_id: DocId,
    /// Cosine-similarity-style score (higher = better, typically `[0,1]`).
    pub score: f64,
    /// Short text excerpt for display.
    pub snippet: String,
    /// Workspace-relative path callers can open.
    pub source_path: SourcePath,
    /// Open-ended metadata passed through from `MemPalace`.
    #[serde(default)]
    pub metadata: Metadata,
}

impl Hit {
    /// Construct a hit, clamping NaN scores to 0.
    #[must_use]
    pub fn new(
        doc_id: impl Into<DocId>,
        score: f64,
        snippet: impl Into<String>,
        source_path: impl Into<SourcePath>,
    ) -> Self {
        Self {
            doc_id: doc_id.into(),
            score: sanitize_score(score),
            snippet: snippet.into(),
            source_path: source_path.into(),
            metadata: Metadata::new(),
        }
    }
}

fn sanitize_score(score: f64) -> f64 {
    if score.is_nan() { 0.0 } else { score }
}

#[cfg(test)]
mod tests {
    use super::Hit;

    #[test]
    fn nan_score_clamps_to_zero() {
        let h = Hit::new("d", f64::NAN, "", "");
        assert!(h.score.is_finite());
        assert!(h.score.abs() < f64::EPSILON);
    }
}
