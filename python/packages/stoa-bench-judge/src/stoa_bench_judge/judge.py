"""LLM judge for scoring free-form benchmark answers."""

from __future__ import annotations

import json

import anthropic

from stoa_bench_judge.cost import CostLedger
from stoa_bench_judge.models import JudgeScore

_DEFAULT_MODEL = "claude-haiku-4-5-20251001"
_MAX_TOKENS = 256
_SYSTEM_PROMPT = (
    "You are an answer correctness judge. "
    "Given a question, a reference answer, and a prediction, "
    "decide if the prediction is correct. "
    'Reply with JSON: {"correct": bool, "confidence": float 0-1, "explanation": str}. '
    "No other text."
)


class LLMJudge:
    """Scores free-form predictions against reference answers using a backbone LLM."""

    def __init__(
        self,
        model: str = _DEFAULT_MODEL,
        ledger: CostLedger | None = None,
    ) -> None:
        """Initialise with a model identifier and optional shared cost ledger."""
        self._client = anthropic.Anthropic()
        self._model = model
        self._ledger = ledger if ledger is not None else CostLedger()

    def score(self, question: str, reference: str, prediction: str) -> JudgeScore:
        """Judge whether `prediction` correctly answers `question` given `reference`."""
        user_msg = f"Question: {question}\nReference answer: {reference}\nPrediction: {prediction}"
        message = self._client.messages.create(
            model=self._model,
            max_tokens=_MAX_TOKENS,
            system=_SYSTEM_PROMPT,
            messages=[{"role": "user", "content": user_msg}],
        )
        self._ledger.record(self._model, message.usage.input_tokens, message.usage.output_tokens)
        return self._parse_response(message)

    @property
    def ledger(self) -> CostLedger:
        """The cost ledger accumulating usage for this judge instance."""
        return self._ledger

    def _parse_response(self, message: anthropic.types.Message) -> JudgeScore:
        raw = ""
        for block in message.content:
            if isinstance(block, anthropic.types.TextBlock):
                raw = block.text
                break
        data: dict[str, object] = json.loads(raw)  # pyright: ignore[reportAny]
        return JudgeScore(
            correct=bool(data.get("correct", False)),
            confidence=float(data.get("confidence", 0.0)),  # type: ignore[arg-type]
            explanation=str(data.get("explanation", "")),
        )
