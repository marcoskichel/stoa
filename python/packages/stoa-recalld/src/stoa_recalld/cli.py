"""`stoa-recalld` entry point.

Parses CLI args (socket path, pid file, log level) and drives
[`serve`][stoa_recalld.server.serve] under asyncio. Designed to be
invoked by `stoa daemon start` via `setsid nohup`.
"""

from __future__ import annotations

import argparse
import asyncio
import logging
from pathlib import Path
import sys

from stoa_recalld.config import DaemonConfig, default_socket_path
from stoa_recalld.server import serve
from stoa_recalld.store import Store


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="stoa-recalld", description="Stoa MemPalace-backed recall daemon"
    )
    parser.add_argument(
        "--foreground", action="store_true", help="Run in the foreground (no daemonize)"
    )
    parser.add_argument(
        "--socket", type=Path, default=default_socket_path(), help="Unix socket path"
    )
    parser.add_argument("--pid-file", type=Path, default=None, help="PID file path")
    parser.add_argument(
        "--log-level",
        default="INFO",
        choices=("DEBUG", "INFO", "WARNING", "ERROR"),
        help="Logging verbosity",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    """Daemon entry point. Returns an exit code suitable for ``sys.exit``."""
    args = _build_parser().parse_args(argv)
    logging.basicConfig(
        level=getattr(logging, str(args.log_level)),
        format="%(asctime)s %(levelname)s %(name)s %(message)s",
    )
    try:
        config = DaemonConfig.resolve(socket_path=args.socket, pid_file=args.pid_file)
    except RuntimeError as e:
        sys.stderr.write(f"stoa-recalld: {e}\n")
        return 2
    store = Store(config)
    try:
        asyncio.run(serve(config.socket_path, store, config.pid_file))
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
