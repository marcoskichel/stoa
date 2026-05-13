//! In-memory shape of a single BEIR subset (corpus + queries + qrels).

use std::collections::HashMap;

/// One document loaded from `corpus.jsonl[.gz]`.
pub(super) struct Document {
    /// BEIR `_id` field; opaque string.
    pub(super) id: String,
    /// Indexable body — `title. text` when both are present.
    pub(super) text: String,
}

/// One evaluation query loaded from `queries.jsonl[.gz]`.
pub(super) struct Query {
    /// BEIR `_id` field; opaque string.
    pub(super) id: String,
    /// Search text — `title. text` when both are present.
    pub(super) text: String,
}

/// `query-id -> { doc-id -> graded relevance }` from `qrels/test.tsv`.
pub(super) type Qrels = HashMap<String, HashMap<String, u32>>;

/// One fully-loaded BEIR subset.
pub(super) struct Corpus {
    pub(super) documents: Vec<Document>,
    pub(super) queries: Vec<Query>,
    pub(super) qrels: Qrels,
}
