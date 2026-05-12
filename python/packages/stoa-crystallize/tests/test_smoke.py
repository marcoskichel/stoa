"""Smoke test for stoa-crystallize."""

from __future__ import annotations

import stoa_crystallize


def test_version_is_not_empty() -> None:
    assert stoa_crystallize.VERSION
