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

/// One ranked retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    /// Stable id: a wiki page id, `raw/<file>`, or `session/<id>:<turn>`.
    pub doc_id: String,
    /// Backend-defined relevance score (already-fused if `streams_matched`
    /// has more than one entry; per-stream raw otherwise).
    pub score: f64,
    /// Short text excerpt for display.
    pub snippet: String,
    /// Path the user can open. Workspace-relative (`wiki/...`,
    /// `sessions/...`) — callers can join the workspace root if they need
    /// an absolute path.
    pub source_path: String,
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
        doc_id: String,
        score: f64,
        snippet: String,
        source_path: String,
        stream: Stream,
    ) -> Self {
        Self {
            doc_id,
            score,
            snippet,
            source_path,
            streams_matched: vec![stream],
            metadata: Metadata::new(),
        }
    }
}
