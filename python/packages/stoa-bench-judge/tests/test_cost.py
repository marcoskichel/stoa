"""Tests for CostLedger."""

from __future__ import annotations

from stoa_bench_judge.cost import CostLedger


def test_empty_ledger_summary() -> None:
    ledger = CostLedger()
    summary = ledger.summary()
    assert summary.total_usd == 0.0
    assert summary.entry_count == 0


def test_record_accumulates_cost() -> None:
    ledger = CostLedger()
    ledger.record("claude-haiku-4-5-20251001", input_tokens=1_000_000, output_tokens=0)
    summary = ledger.summary()
    assert summary.entry_count == 1
    assert abs(summary.total_usd - 0.80) < 0.001


def test_unknown_model_uses_default_rates() -> None:
    ledger = CostLedger()
    ledger.record("unknown-model-xyz", input_tokens=1_000_000, output_tokens=0)
    summary = ledger.summary()
    assert summary.total_usd > 0.0


def test_multiple_entries_sum() -> None:
    ledger = CostLedger()
    ledger.record("claude-haiku-4-5-20251001", input_tokens=500_000, output_tokens=500_000)
    ledger.record("claude-haiku-4-5-20251001", input_tokens=500_000, output_tokens=500_000)
    summary = ledger.summary()
    assert summary.entry_count == 2
    assert summary.total_usd > 0.0
