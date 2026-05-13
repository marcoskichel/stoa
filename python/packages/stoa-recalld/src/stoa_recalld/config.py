"""Runtime configuration for stoa-recalld.

Resolves the workspace root (nearest ``STOA.md`` ancestor), the
MemPalace palace path, and the Unix socket the daemon binds to.
"""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path


def _xdg_runtime_dir() -> Path:
    """Best-effort XDG runtime dir."""
    explicit = os.environ.get("XDG_RUNTIME_DIR")
    if explicit:
        return Path(explicit)
    user = os.environ.get("USER", "default")
    return Path(f"/tmp/stoa-runtime-{user}")


def _xdg_data_home() -> Path:
    """Best-effort XDG data home."""
    explicit = os.environ.get("XDG_DATA_HOME")
    if explicit:
        return Path(explicit)
    return Path.home() / ".local" / "share"


def default_socket_path() -> Path:
    """Match the Rust client's `default_socket_path()` resolution order."""
    explicit = os.environ.get("STOA_RECALLD_SOCKET")
    if explicit:
        return Path(explicit)
    return _xdg_runtime_dir() / "stoa-recalld.sock"


def default_palace_path(workspace_root: Path) -> Path:
    """Per-workspace palace under ``<workspace>/.stoa/palace``.

    Stoa keeps each workspace's MemPalace data segregated under the
    workspace's `.stoa/` dir, mirroring how the audit log lives there.
    """
    explicit = os.environ.get("STOA_PALACE_PATH")
    if explicit:
        return Path(explicit)
    return workspace_root / ".stoa" / "palace"


def default_wiki_dir(workspace_root: Path) -> Path:
    """Canonical wiki dir, relative to workspace root."""
    return workspace_root / "wiki"


def find_workspace_root(start: Path) -> Path | None:
    """Walk up from ``start`` looking for ``STOA.md``."""
    cursor: Path | None = start.resolve()
    while cursor is not None:
        if (cursor / "STOA.md").is_file():
            return cursor
        parent = cursor.parent
        if parent == cursor:
            return None
        cursor = parent
    return None


@dataclass(frozen=True)
class DaemonConfig:
    """Resolved daemon configuration."""

    workspace_root: Path
    palace_path: Path
    wiki_dir: Path
    socket_path: Path
    pid_file: Path | None

    @classmethod
    def resolve(cls, socket_path: Path, pid_file: Path | None) -> DaemonConfig:
        """Build a config from the current working directory + env vars."""
        root = find_workspace_root(Path.cwd())
        if root is None:
            msg = (
                "No STOA.md found from cwd up to /. Run `stoa init` in the "
                "workspace before starting the daemon."
            )
            raise RuntimeError(msg)
        return cls(
            workspace_root=root,
            palace_path=default_palace_path(root),
            wiki_dir=default_wiki_dir(root),
            socket_path=socket_path,
            pid_file=pid_file,
        )
