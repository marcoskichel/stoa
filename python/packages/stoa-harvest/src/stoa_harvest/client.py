"""Tiny synchronous client for the stoa-recalld Unix socket.

Used by harvest + crystallize to talk to the daemon without pulling in
asyncio. One request per connection.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import socket
from typing import Any, cast


class DaemonError(RuntimeError):
    """Raised when the daemon returns ``ok=false`` or the socket dies."""


def default_socket_path() -> Path:
    """Mirror Rust client's default-socket-path resolution."""
    explicit = os.environ.get("STOA_RECALLD_SOCKET")
    if explicit:
        return Path(explicit)
    runtime = os.environ.get("XDG_RUNTIME_DIR")
    if runtime:
        return Path(runtime) / "stoa-recalld.sock"
    user = os.environ.get("USER", "default")
    return Path(f"/tmp/stoa-recalld-{user}.sock")  # noqa: S108


def rpc(method: str, params: dict[str, Any], socket_path: Path | None = None) -> dict[str, Any]:
    """Issue one RPC. Raises `DaemonError` on transport or ok=false."""
    sock_path = socket_path or default_socket_path()
    payload = (json.dumps({"method": method, "params": params}) + "\n").encode("utf-8")
    response = _roundtrip(sock_path, payload)
    return _parse_envelope(response)


def _roundtrip(sock_path: Path, payload: bytes) -> str:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(10.0)
        try:
            s.connect(str(sock_path))
        except (FileNotFoundError, ConnectionRefusedError) as e:
            msg = f"stoa-recalld socket {sock_path} not reachable: {e}"
            raise DaemonError(msg) from e
        s.sendall(payload)
        try:
            s.shutdown(socket.SHUT_WR)
        except OSError:
            pass
        chunks: list[bytes] = []
        while True:
            chunk = s.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
    return b"".join(chunks).decode("utf-8").strip()


def _parse_envelope(response: str) -> dict[str, Any]:
    if not response:
        msg = "daemon returned empty response"
        raise DaemonError(msg)
    try:
        parsed = cast("dict[str, Any]", json.loads(response))
    except json.JSONDecodeError as e:
        msg = f"daemon returned non-JSON: {response[:200]}"
        raise DaemonError(msg) from e
    if not parsed.get("ok"):
        err = cast("dict[str, Any]", parsed.get("error") or {})
        code = err.get("code", "unknown")
        message = err.get("message", "")
        msg = f"daemon error [{code}]: {message}"
        raise DaemonError(msg)
    result = parsed.get("result")
    if not isinstance(result, dict):
        msg = "daemon ok=true without result"
        raise DaemonError(msg)
    return cast("dict[str, Any]", result)
