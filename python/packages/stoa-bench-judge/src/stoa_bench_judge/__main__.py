"""CLI entry point for stoa-bench-judge."""

from __future__ import annotations

import argparse
import json
import sys

from stoa_bench_judge.judge import LLMJudge

_SUBCOMMANDS = frozenset({"judge"})


def main() -> None:
    """Entry point: stoa-bench-judge judge --question Q --reference R --prediction P."""
    parser = _build_parser()
    args = parser.parse_args()
    command: str = args.__dict__.get("command") or ""  # pyright: ignore[reportAny]
    if command == "judge":
        _run_judge(args)
    else:
        parser.print_help()
        sys.exit(1)


def _build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser with subcommands."""
    parser = argparse.ArgumentParser(prog="stoa-bench-judge")
    sub = parser.add_subparsers(dest="command")
    judge_p = sub.add_parser("judge", help="Score a prediction against a reference answer")
    judge_p.add_argument("--question", required=True)
    judge_p.add_argument("--reference", required=True)
    judge_p.add_argument("--prediction", required=True)
    judge_p.add_argument("--model", default="claude-haiku-4-5-20251001")
    return parser


def _run_judge(args: argparse.Namespace) -> None:
    """Execute the judge subcommand and write JSON result to stdout."""
    d = args.__dict__  # pyright: ignore[reportAny]
    model: str = str(d.get("model", "claude-haiku-4-5-20251001"))  # pyright: ignore[reportAny]
    question: str = str(d.get("question", ""))  # pyright: ignore[reportAny]
    reference: str = str(d.get("reference", ""))  # pyright: ignore[reportAny]
    prediction: str = str(d.get("prediction", ""))  # pyright: ignore[reportAny]
    judge = LLMJudge(model=model)
    score = judge.score(question=question, reference=reference, prediction=prediction)
    payload = {
        "correct": score.correct,
        "confidence": score.confidence,
        "explanation": score.explanation,
    }
    sys.stdout.write(json.dumps(payload) + "\n")
