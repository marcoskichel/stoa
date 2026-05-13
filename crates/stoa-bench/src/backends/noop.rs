//! Zero-recall control backend — always returns empty results.

use std::path::Path;

use async_trait::async_trait;
use stoa_recall::{Filters, Hit, RecallBackend, RecallError, StreamSet};

/// Zero-recall control backend.
///
/// Implements [`RecallBackend`] as a no-op: every read returns an empty
/// hit list, every write succeeds without storing anything. Used as the
/// control arm for "delta vs no recall at the same backbone" comparisons.
pub(crate) struct NoopBackend;

#[async_trait]
impl RecallBackend for NoopBackend {
    async fn index_page(
        &self,
        _page_id: &str,
        _content: &str,
        _source_path: &str,
        _metadata: &serde_json::Value,
    ) -> Result<(), RecallError> {
        Ok(())
    }

    async fn index_session(
        &self,
        _session_id: &str,
        _jsonl_path: &Path,
    ) -> Result<(), RecallError> {
        Ok(())
    }

    async fn remove(&self, _doc_id: &str) -> Result<(), RecallError> {
        Ok(())
    }

    async fn search(
        &self,
        _query: &str,
        _k: usize,
        _filters: &Filters,
        _streams: StreamSet,
    ) -> Result<Vec<Hit>, RecallError> {
        Ok(vec![])
    }

    async fn health_check(&self) -> Result<serde_json::Value, RecallError> {
        Ok(serde_json::json!({"backend": "noop"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_search_returns_empty() {
        let backend = NoopBackend;
        let result = backend
            .search("anything", 10, &Filters::default(), StreamSet::all())
            .await;
        assert!(result.is_ok(), "noop backend must not fail");
        assert!(result.unwrap_or_default().is_empty());
    }
}
