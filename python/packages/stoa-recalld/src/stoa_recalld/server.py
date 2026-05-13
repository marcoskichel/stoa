"""Asyncio Unix-socket server for stoa-recalld.

One JSON-line request per connection: client writes, server reads to
EOF / newline, server writes one response line, server closes. The
underlying [`Store`][stoa_recalld.store.Store] is shared across
connections; mempalace queries are thread-safe via ChromaDB's HNSW
read/write locks.
"""

from __future__ import annotations

import asyncio
from contextlib import suppress
import json
import logging
from pathlib import Path
import signal
from typing import TYPE_CHECKING, Any

from stoa_recalld.handlers import HandlerError, handle

if TYPE_CHECKING:
    from stoa_recalld.store import Store

logger = logging.getLogger("stoa_recalld")


async def serve(socket_path: Path, store: Store, pid_file: Path | None = None) -> None:
    """Bind a Unix-domain socket and serve until SIGTERM/SIGINT."""
    if socket_path.exists():
        socket_path.unlink()
    socket_path.parent.mkdir(parents=True, exist_ok=True)

    server = await asyncio.start_unix_server(
        lambda r, w: _handle_connection(r, w, store),
        path=str(socket_path),
    )
    if pid_file is not None:
        pid_file.parent.mkdir(parents=True, exist_ok=True)
        pid_file.write_text(str(_pid()))
    stop_event = asyncio.Event()
    _install_signal_handlers(stop_event)
    logger.info("stoa-recalld listening on %s", socket_path)
    try:
        async with server:
            await stop_event.wait()
    finally:
        with suppress(FileNotFoundError):
            socket_path.unlink()
        if pid_file is not None:
            with suppress(FileNotFoundError):
                pid_file.unlink()


def _pid() -> int:
    import os

    return os.getpid()


def _install_signal_handlers(stop: asyncio.Event) -> None:
    loop = asyncio.get_running_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, stop.set)


async def _handle_connection(
    reader: asyncio.StreamReader,
    writer: asyncio.StreamWriter,
    store: Store,
) -> None:
    """Read one JSON line, dispatch, write one JSON response, close."""
    try:
        line = await reader.readline()
        if not line:
            return
        await _process_line(line, writer, store)
    except (ConnectionResetError, BrokenPipeError):
        return
    finally:
        with suppress(ConnectionResetError, BrokenPipeError):
            writer.close()
            await writer.wait_closed()


async def _process_line(
    line: bytes,
    writer: asyncio.StreamWriter,
    store: Store,
) -> None:
    response: dict[str, Any]
    try:
        envelope = json.loads(line.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as e:
        response = {"ok": False, "error": {"code": "bad_request", "message": str(e)}}
    else:
        if not isinstance(envelope, dict):
            response = {
                "ok": False,
                "error": {"code": "bad_request", "message": "envelope must be an object"},
            }
        else:
            response = _dispatch(envelope, store)
    payload = json.dumps(response).encode("utf-8") + b"\n"
    try:
        writer.write(payload)
        await writer.drain()
    except (ConnectionResetError, BrokenPipeError):
        return


def _dispatch(envelope: dict[str, Any], store: Store) -> dict[str, Any]:
    method = envelope.get("method")
    params = envelope.get("params") or {}
    if not isinstance(method, str) or not isinstance(params, dict):
        return {"ok": False, "error": {"code": "bad_request", "message": "missing method/params"}}
    try:
        result = handle(method, params, store)
    except HandlerError as e:
        return {"ok": False, "error": {"code": e.code, "message": e.message}}
    except RuntimeError as e:
        return {"ok": False, "error": {"code": "internal", "message": str(e)}}
    return {"ok": True, "result": result}
