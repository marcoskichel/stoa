"""Smoke test for stoa-shared."""

from __future__ import annotations

import stoa_shared


def test_version_is_not_empty() -> None:
    assert stoa_shared.VERSION
