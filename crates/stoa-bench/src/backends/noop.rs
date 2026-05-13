use std::path::Path;

use stoa_recall::{Hit, RecallBackend, RecallError, SearchParams};

/// Zero-recall control backend.
///
/// Always returns an empty hit list. Used as the control arm for computing
/// "delta vs no recall at the same backbone" — the headline metric that
/// justifies Stoa's existence.
pub(crate) struct NoopBackend;

impl RecallBackend for NoopBackend {
    fn search(&self, _params: &SearchParams) -> Result<Vec<Hit>, RecallError> {
        Ok(vec![])
    }

    fn index_path(&self, _path: &Path) -> Result<(), RecallError> {
        Ok(())
    }

    fn rebuild(&self) -> Result<(), RecallError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stoa_recall::SearchParams;

    #[test]
    fn noop_search_returns_empty() {
        let backend = NoopBackend;
        let params = SearchParams {
            query: "anything".to_owned(),
            k: 10,
            streams: vec![],
        };
        let result = backend.search(&params);
        assert!(result.is_ok(), "noop backend must not fail");
        assert!(result.unwrap_or_default().is_empty());
    }
}
