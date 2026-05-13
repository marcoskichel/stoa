"""Recall worker entry point.

Run as `python -m stoa_recall.worker --queue <path>`; loops claiming
rows from `recall.request`, dispatching to the LocalChromaSqliteBackend,
and writing `recall.response`. ChromaDB + fastembed are imported lazily
on first hybrid query so a BM25-only workspace never pays the cost.

M4 ships the IPC + dispatch surface — concrete ChromaDB integration
lands in M5 alongside harvest + crystallize.
"""

from __future__ import annotations

import argparse
import logging
import sys

logger: logging.Logger = logging.getLogger(__name__)


class WorkerArgs:
    """Parsed CLI arguments for the worker."""

    def __init__(self, queue: str, *, once: bool) -> None:
        """Capture the parsed CLI args."""
        self.queue: str = queue
        self.once: bool = once


def parse_args(argv: list[str]) -> WorkerArgs:
    """Parse `--queue` + `--once` flags into a typed bag."""
    parser = argparse.ArgumentParser(prog="stoa_recall.worker")
    _ = parser.add_argument("--queue", type=str, required=True, help="Path to queue.db")
    _ = parser.add_argument(
        "--once",
        action="store_true",
        help="Drain one row and exit (test/CI hook).",
    )
    raw = parser.parse_args(argv)
    queue: str = str(raw.queue)  # type: ignore[reportAny]
    once: bool = bool(raw.once)  # type: ignore[reportAny]
    return WorkerArgs(queue=queue, once=once)


def main(argv: list[str] | None = None) -> int:
    """Worker entry point."""
    args = parse_args(argv if argv is not None else sys.argv[1:])
    logging.basicConfig(level=logging.INFO)
    logger.info("stoa_recall worker starting (queue=%s once=%s)", args.queue, args.once)
    logger.warning("recall worker stub running; Rust IpcBackend will degrade to BM25-only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
