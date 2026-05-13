//! Stoa `RecallBackend` trait + reciprocal rank fusion.
//!
//! Spec source: [ARCHITECTURE.md §6.1] (`RecallBackend` interface).
//!
//! This crate is the substrate-free contract layer. Concrete backends live
//! in sibling crates (`stoa-recall-local-chroma-sqlite` is the v0.1
//! default). Anything that talks to a `SQLite` or vector store belongs
//! there, not here.

mod fusion;
mod hit;
mod stream;
mod traits;

pub use fusion::{RRF_K, rrf_fuse};
pub use hit::{DocId, Hit, Metadata, SourcePath};
pub use stream::{Stream, StreamSet};
pub use traits::{Filters, RecallBackend, RecallError, RecallResult};
