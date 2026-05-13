"""Stoa recall sidecar — LocalChromaSqliteBackend over the queue.

Spec source: ARCHITECTURE.md §6.1.

The Rust IpcBackend writes recall.request rows; the worker entry point
in this package (`python -m stoa_recall.worker`) drains them and writes
recall.response rows. Heavy runtime deps (chromadb, fastembed) are
imported lazily inside the worker to keep the package importable in
type-check / test environments.
"""

from __future__ import annotations

VERSION: str = "0.1.0"
