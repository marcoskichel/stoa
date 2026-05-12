"""Smoke test for stoa-embed."""

from __future__ import annotations

import stoa_embed


def test_version_is_not_empty() -> None:
    assert stoa_embed.VERSION
