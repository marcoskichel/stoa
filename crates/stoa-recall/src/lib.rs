//! Stoa `RecallBackend` trait + reciprocal rank fusion.

mod backend;
mod error;
mod types;

pub use backend::RecallBackend;
pub use error::RecallError;
pub use types::{Hit, SearchParams, Stream};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_version_is_not_empty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
