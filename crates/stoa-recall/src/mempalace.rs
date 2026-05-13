//! `MempalaceBackend` — Unix socket client for [`stoa-recalld`].
//!
//! One connection per call: connect → write one JSON line → read one
//! JSON line → close. Stateless on the Rust side; the daemon does all
//! state management around the underlying `MemPalace` palace.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

use crate::hit::Hit;
use crate::traits::{Filters, RecallBackend, RecallError, RecallResult};
use crate::wire::{
    HealthResponse, MineRequest, MineResponse, ReadWikiRequest, ReadWikiResponse, SearchRequest,
    SearchResponse, WriteWikiRequest, WriteWikiResponse,
};

/// Default per-call deadline. `UserPromptSubmit` hooks budget ~500ms for
/// retrieval; the daemon should answer warm in <150ms, cold in <500ms.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// Default socket path: `$XDG_RUNTIME_DIR/stoa-recalld.sock`, fallback
/// `/tmp/stoa-recalld-$USER.sock`. Override via `STOA_RECALLD_SOCKET` env.
#[must_use]
pub fn default_socket_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("STOA_RECALLD_SOCKET") {
        return PathBuf::from(explicit);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime_dir).join("stoa-recalld.sock");
        return p;
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_owned());
    PathBuf::from(format!("/tmp/stoa-recalld-{user}.sock"))
}

/// Unix-socket-backed `RecallBackend` adapter.
#[derive(Debug, Clone)]
pub struct MempalaceBackend {
    socket_path: PathBuf,
    deadline: Duration,
}

impl MempalaceBackend {
    /// Build a backend pointed at the given socket path.
    #[must_use]
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            deadline: DEFAULT_TIMEOUT,
        }
    }

    /// Build a backend using [`default_socket_path`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(default_socket_path())
    }

    /// Override the per-call deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Reveal the underlying socket path.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    async fn rpc<P, R>(&self, method: &str, params: P) -> RecallResult<R>
    where
        P: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let request = serde_json::json!({
            "method": method,
            "params": params,
        });
        let payload = serde_json::to_vec(&request)?;
        let fut = self.roundtrip(&payload);
        let raw = match timeout(self.deadline, fut).await {
            Ok(inner) => inner?,
            Err(_) => {
                return Err(RecallError::DeadlineExceeded {
                    millis: u64::try_from(self.deadline.as_millis()).unwrap_or(u64::MAX),
                });
            },
        };
        Self::parse_response(&raw)
    }

    async fn roundtrip(&self, payload: &[u8]) -> RecallResult<Vec<u8>> {
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) {
                RecallError::Unavailable(format!(
                    "daemon socket {} not reachable ({e}). Start with `stoa daemon start`.",
                    self.socket_path.display()
                ))
            } else {
                RecallError::Io(e)
            }
        })?;
        let (read_half, mut write_half) = stream.into_split();
        write_half.write_all(payload).await?;
        write_half.write_all(b"\n").await?;
        write_half.shutdown().await?;
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        let _bytes = reader.read_line(&mut line).await?;
        Ok(line.into_bytes())
    }

    fn parse_response<R>(raw: &[u8]) -> RecallResult<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let env: serde_json::Value = serde_json::from_slice(raw)?;
        let ok = env
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if ok {
            let result_value = env.get("result").ok_or_else(|| RecallError::Daemon {
                code: "no_result".to_owned(),
                message: "daemon returned ok=true without result".to_owned(),
            })?;
            Ok(serde_json::from_value(result_value.clone())?)
        } else {
            let err = env.get("error");
            let code = err
                .and_then(|e| e.get("code"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            let message = err
                .and_then(|e| e.get("message"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("daemon returned ok=false without error body")
                .to_owned();
            Err(RecallError::Daemon { code, message })
        }
    }
}

#[async_trait]
impl RecallBackend for MempalaceBackend {
    async fn search(&self, query: &str, top_k: usize, filters: &Filters) -> RecallResult<Vec<Hit>> {
        let req = SearchRequest {
            query: query.to_owned(),
            top_k,
            filters: filters.eq.clone(),
        };
        let resp: SearchResponse = self.rpc("search", req).await?;
        Ok(resp.hits)
    }

    async fn mine(&self, source_file: &str) -> RecallResult<Vec<String>> {
        let req = MineRequest {
            source_file: source_file.to_owned(),
        };
        let resp: MineResponse = self.rpc("mine", req).await?;
        Ok(resp.drawer_ids)
    }

    async fn write_wiki(
        &self,
        page_id: &str,
        frontmatter: &serde_json::Value,
        body: &str,
    ) -> RecallResult<String> {
        let req = WriteWikiRequest {
            page_id: page_id.to_owned(),
            frontmatter: frontmatter.clone(),
            body: body.to_owned(),
        };
        let resp: WriteWikiResponse = self.rpc("write_wiki", req).await?;
        Ok(resp.path)
    }

    async fn read_wiki(&self, page_id: &str) -> RecallResult<(serde_json::Value, String)> {
        let req = ReadWikiRequest { page_id: page_id.to_owned() };
        let resp: ReadWikiResponse = self.rpc("read_wiki", req).await?;
        Ok((resp.frontmatter, resp.body))
    }

    async fn health(&self) -> RecallResult<serde_json::Value> {
        let resp: HealthResponse = self.rpc("health", serde_json::json!({})).await?;
        Ok(serde_json::to_value(resp)?)
    }
}

#[cfg(test)]
mod tests {
    use super::MempalaceBackend;

    #[test]
    fn backend_records_its_socket_path() {
        let b = MempalaceBackend::new("/tmp/stoa-test-pivot.sock");
        assert_eq!(b.socket_path().to_string_lossy(), "/tmp/stoa-test-pivot.sock");
    }
}
