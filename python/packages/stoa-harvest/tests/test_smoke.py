"""Smoke test for stoa-harvest."""

from __future__ import annotations

import stoa_harvest


def test_version_is_not_empty() -> None:
    assert stoa_harvest.VERSION
