//! `Hit` — one ranked retrieval result.
//!
//! The shape mirrors the Python `Hit` dataclass in `ARCHITECTURE` §6.1:
//! doc id + score + snippet + always-resolvable `source_path` + per-stream
//! provenance. `metadata` is open-ended; backends populate it with whatever
//! per-doc fields callers should not have to re-fetch.

use serde::{Deserialize, Serialize};

use crate::stream::Stream;

/// Open-ended metadata bag attached to every [`Hit`].
///
/// Stored as a `serde_json::Map<String, serde_json::Value>` so callers can
/// pass it through to JSON output without round-tripping a typed struct.
pub type Metadata = serde_json::Map<String, serde_json::Value>;

/// Stable doc id (a wiki page id, `raw/<file>`, `session/<id>:<turn>`).
///
/// Newtyped so backends and downstream code cannot mix it up with a
/// `SourcePath` or a free-form `String` at a callsite. Serializes as a
/// bare JSON string for wire compatibility.
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

impl PartialEq<str> for DocId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for DocId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Workspace-relative path the user can open (`wiki/...`,
/// `sessions/...`, `raw/...`). Callers can join the workspace root for
/// an absolute path.
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

impl PartialEq<str> for SourcePath {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for SourcePath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// One ranked retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    /// Stable id: a wiki page id, `raw/<file>`, or `session/<id>:<turn>`.
    pub doc_id: DocId,
    /// Backend-defined relevance score (already-fused if `streams_matched`
    /// has more than one entry; per-stream raw otherwise).
    pub score: f64,
    /// Short text excerpt for display.
    pub snippet: String,
    /// Path the user can open. Workspace-relative (`wiki/...`,
    /// `sessions/...`) — callers can join the workspace root if they need
    /// an absolute path.
    pub source_path: SourcePath,
    /// Streams that contributed to this hit. Iteration order is
    /// `[vector, bm25, graph]` for callers serializing to JSON.
    pub streams_matched: Vec<Stream>,
    /// Optional per-doc metadata (kind, type, neighbors, etc.).
    #[serde(default)]
    pub metadata: Metadata,
}

impl Hit {
    /// Convenience for backends building a hit from a single stream.
    #[must_use]
    pub fn single_stream(
        doc_id: impl Into<DocId>,
        score: f64,
        snippet: impl Into<String>,
        source_path: impl Into<SourcePath>,
        stream: Stream,
    ) -> Self {
        Self {
            doc_id: doc_id.into(),
            score: sanitize_score(score),
            snippet: snippet.into(),
            source_path: source_path.into(),
            streams_matched: vec![stream],
            metadata: Metadata::new(),
        }
    }
}

/// Reject NaN by mapping it to `0.0`.
///
/// RRF fusion sorts by score; NaN propagates through `partial_cmp` as
/// `None`, which would scramble the ordering and silently drop the
/// hit to the bottom of the result list. Callers should never see a
/// NaN score, so the safest move at the constructor is to clamp it.
fn sanitize_score(score: f64) -> f64 {
    if score.is_nan() { 0.0 } else { score }
}

#[cfg(test)]
mod tests {
    use super::Hit;
    use crate::stream::Stream;

    #[test]
    fn single_stream_clamps_nan_score_to_zero() {
        let hit = Hit::single_stream("d", f64::NAN, "", "", Stream::Bm25);
        assert!(hit.score.is_finite(), "NaN must be clamped: {}", hit.score);
        assert!(hit.score.abs() < f64::EPSILON, "expected 0.0, got {}", hit.score);
    }
}
