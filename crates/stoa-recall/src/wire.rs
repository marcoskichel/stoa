//! JSON-line wire protocol between the Rust clients and `stoa-recalld`.
//!
//! Newline-delimited JSON over Unix domain socket. One request per
//! connection — daemon writes one response and closes the stream. This
//! keeps the Rust client free of stateful protocol concerns (no IDs, no
//! correlation tables) at the cost of a fresh handshake per call. The
//! cost is small: ~1 ms per fresh `connect()` on a warm daemon.
//!
//! Methods: `search`, `mine`, `write_wiki`, `read_wiki`, `health`.
//!
//! Every request has the shape `{"method": "...", "params": {...}}` and
//! every response is `{"ok": true, "result": {...}}` or
//! `{"ok": false, "error": {"code": "...", "message": "..."}}`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::hit::Hit;

/// Search request body.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Free-text query.
    pub query: String,
    /// Max hits to return.
    pub top_k: usize,
    /// Inclusive equality filters (`kind=wiki`, `wing=X`, etc.).
    #[serde(default)]
    pub filters: BTreeMap<String, String>,
}

/// Search response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Ranked hits, highest score first.
    pub hits: Vec<Hit>,
}

/// Mine request body.
#[derive(Debug, Serialize, Deserialize)]
pub struct MineRequest {
    /// Path to a transcript JSONL file or arbitrary text source.
    pub source_file: String,
}

/// Mine response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct MineResponse {
    /// IDs of drawers created (or already present, if idempotent).
    pub drawer_ids: Vec<String>,
}

/// Write-wiki request body.
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteWikiRequest {
    /// Stable page id (e.g. `ent-redis`).
    pub page_id: String,
    /// YAML frontmatter, marshalled as JSON object.
    pub frontmatter: serde_json::Value,
    /// Body markdown.
    pub body: String,
}

/// Write-wiki response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteWikiResponse {
    /// Workspace-relative path of the written page.
    pub path: String,
}

/// Read-wiki request body.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadWikiRequest {
    /// Stable page id.
    pub page_id: String,
}

/// Read-wiki response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadWikiResponse {
    /// YAML frontmatter as JSON.
    pub frontmatter: serde_json::Value,
    /// Body markdown.
    pub body: String,
    /// Workspace-relative path.
    pub path: String,
}

/// Health response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// `"ok"` when the daemon can reach its mempalace palace.
    pub status: String,
    /// Absolute path of the active mempalace palace.
    pub palace_path: String,
    /// `MemPalace` package version reported by the daemon.
    pub mempalace_version: String,
}
