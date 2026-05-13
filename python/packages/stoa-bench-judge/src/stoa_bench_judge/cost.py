"""Token cost ledger for accumulating API costs across benchmark runs."""

from __future__ import annotations

from stoa_bench_judge.models import CostEntry, LedgerSummary

_COST_PER_1M_INPUT: dict[str, float] = {
    "claude-haiku-4-5-20251001": 0.80,
    "claude-sonnet-4-6": 3.00,
    "claude-opus-4-7": 15.00,
}
_COST_PER_1M_OUTPUT: dict[str, float] = {
    "claude-haiku-4-5-20251001": 4.00,
    "claude-sonnet-4-6": 15.00,
    "claude-opus-4-7": 75.00,
}
_DEFAULT_INPUT_RATE: float = 3.00
_DEFAULT_OUTPUT_RATE: float = 15.00
_TOKENS_PER_MILLION: int = 1_000_000


class CostLedger:
    """Accumulates token costs across all LLM calls in a benchmark run."""

    def __init__(self) -> None:
        """Initialise an empty ledger."""
        self._entries: list[CostEntry] = []

    def record(self, model: str, input_tokens: int, output_tokens: int) -> None:
        """Append a cost entry for one LLM call."""
        cost = self._compute_cost(model, input_tokens, output_tokens)
        self._entries.append(CostEntry(model, input_tokens, output_tokens, cost))

    def summary(self) -> LedgerSummary:
        """Return aggregated cost across all recorded calls."""
        return LedgerSummary(
            total_usd=sum(e.cost_usd for e in self._entries),
            entry_count=len(self._entries),
        )

    def _compute_cost(self, model: str, inp: int, out: int) -> float:
        i_rate = _COST_PER_1M_INPUT.get(model, _DEFAULT_INPUT_RATE)
        o_rate = _COST_PER_1M_OUTPUT.get(model, _DEFAULT_OUTPUT_RATE)
        return (inp * i_rate + out * o_rate) / _TOKENS_PER_MILLION
