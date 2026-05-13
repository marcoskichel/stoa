use serde::{Deserialize, Serialize};

/// A document fragment returned by a recall search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    /// Workspace-relative path to the source document.
    pub source_path: String,
    /// Verbatim text excerpt from the source document.
    pub content: String,
    /// Reciprocal rank fusion score in [0.0, 1.0].
    pub score: f32,
    /// Retrieval streams that contributed to this hit.
    pub streams: Vec<Stream>,
}

/// Retrieval stream identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stream {
    /// BM25 sparse full-text index.
    Bm25,
    /// Dense vector embedding index.
    Vector,
    /// Typed knowledge graph.
    Graph,
}

/// Parameters for a hybrid recall search.
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// Query text.
    pub query: String,
    /// Maximum number of hits to return.
    pub k: usize,
    /// Streams to query; empty means all available streams.
    pub streams: Vec<Stream>,
}
