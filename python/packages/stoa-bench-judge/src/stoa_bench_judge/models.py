"""Shared data models for the judge and cost ledger."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class JudgeScore:
    """Result of a single LLM-judged answer comparison."""

    correct: bool
    confidence: float
    explanation: str


@dataclass(frozen=True)
class CostEntry:
    """Token cost record for one LLM call."""

    model: str
    input_tokens: int
    output_tokens: int
    cost_usd: float


@dataclass(frozen=True)
class LedgerSummary:
    """Aggregated cost across all judge calls in a run."""

    total_usd: float
    entry_count: int
