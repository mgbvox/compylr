"""Shared fixtures.

The manager is process-wide by design — a project compiles to one shared artifact — which is
exactly what independent test cases cannot tolerate. Every test that touches it therefore resets
it, and anything that builds gets its own directory so one test's cache cannot answer another
test's question.
"""

from __future__ import annotations

import shutil
from collections.abc import Iterator
from pathlib import Path

import pytest
from compylr import _manager


@pytest.fixture(autouse=True)
def reset_manager() -> Iterator[None]:
    """Drop the process-wide manager around every test."""
    _manager._reset_for_tests()
    yield
    _manager._reset_for_tests()


@pytest.fixture
def build_root(tmp_path: Path) -> Path:
    """An isolated artifact directory."""
    return tmp_path / ".compylr"


def toolchain_available() -> bool:
    """Whether this machine can actually build a generated crate."""
    return shutil.which("cargo") is not None and shutil.which("maturin") is not None


#: Skips tests that compile Rust when the toolchain is absent, so a machine without it still gets
#: a meaningful run of everything else rather than a wall of errors.
needs_toolchain = pytest.mark.skipif(
    not toolchain_available(),
    reason="requires cargo and maturin",
)
