//! Stoa recall — backend trait + `MemPalace` adapter.
//!
//! Stoa's only retrieval backend is the long-lived Python daemon
//! [`stoa-recalld`] which hosts `MemPalace` in-process. This crate is the
//! Rust-side contract: a `RecallBackend` trait, a `Hit` value type that
//! mirrors what the daemon emits, and a `MempalaceBackend` that talks to
//! the daemon over a Unix domain socket using newline-delimited JSON.
//!
//! Spec: ARCHITECTURE.md §Overview (post-pivot, 2026-05-13).

pub mod hit;
pub mod mempalace;
pub mod traits;
pub mod wire;

pub use hit::{DocId, Hit, Metadata, SourcePath};
pub use mempalace::{MempalaceBackend, default_socket_path};
pub use traits::{Filters, RecallBackend, RecallError, RecallResult};
pub use wire::{
    HealthResponse, MineRequest, MineResponse, ReadWikiRequest, ReadWikiResponse, SearchRequest,
    SearchResponse, WriteWikiRequest, WriteWikiResponse,
};
