//! Recall streams + per-call selection.
//!
//! `ARCHITECTURE` §6.1 names three streams: `vector`, `bm25`, `graph`.
//! Each [`Stream`] variant serializes to its lower-case spelling so JSON
//! shapes match the wire format the Python sidecar emits.

use serde::{Deserialize, Serialize};

/// One of the three retrieval streams the backend may consult.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    /// Vector / embedding similarity (`ChromaDB` in the default backend).
    Vector,
    /// BM25 keyword search (`SQLite` FTS5 in the default backend).
    Bm25,
    /// Knowledge-graph traversal (`SQLite` KG tables).
    Graph,
}

impl Stream {
    /// Wire-name. Matches the JSON serialization used in `streams_matched`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Bm25 => "bm25",
            Self::Graph => "graph",
        }
    }

    /// Parse the wire-name back into a [`Stream`].
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "vector" => Some(Self::Vector),
            "bm25" => Some(Self::Bm25),
            "graph" => Some(Self::Graph),
            _ => None,
        }
    }
}

/// Set of streams the caller wants the backend to consult.
///
/// Iteration is deterministic in `[Vector, Bm25, Graph]` order so RRF
/// fusion + JSON serialization are stable across calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamSet {
    vector: bool,
    bm25: bool,
    graph: bool,
}

impl StreamSet {
    /// All three streams active — the default for hybrid recall.
    #[must_use]
    pub fn all() -> Self {
        Self {
            vector: true,
            bm25: true,
            graph: true,
        }
    }

    /// BM25 only — the no-embeddings fallback path.
    #[must_use]
    pub fn bm25_only() -> Self {
        Self {
            vector: false,
            bm25: true,
            graph: false,
        }
    }

    /// Construct from an explicit slice of streams.
    #[must_use]
    pub fn from_slice(streams: &[Stream]) -> Self {
        let mut s = Self {
            vector: false,
            bm25: false,
            graph: false,
        };
        for stream in streams {
            s.set(*stream);
        }
        s
    }

    /// True when no stream is selected (caller bug — backend should reject).
    #[must_use]
    pub fn is_empty(self) -> bool {
        !self.vector && !self.bm25 && !self.graph
    }

    /// Test whether `stream` is included.
    #[must_use]
    pub fn contains(self, stream: Stream) -> bool {
        match stream {
            Stream::Vector => self.vector,
            Stream::Bm25 => self.bm25,
            Stream::Graph => self.graph,
        }
    }

    /// Add `stream` to the set.
    pub fn set(&mut self, stream: Stream) {
        match stream {
            Stream::Vector => self.vector = true,
            Stream::Bm25 => self.bm25 = true,
            Stream::Graph => self.graph = true,
        }
    }

    /// Iterate selected streams in canonical order.
    pub fn iter(self) -> impl Iterator<Item = Stream> {
        [Stream::Vector, Stream::Bm25, Stream::Graph]
            .into_iter()
            .filter(move |s| self.contains(*s))
    }
}

impl Default for StreamSet {
    fn default() -> Self {
        Self::all()
    }
}
