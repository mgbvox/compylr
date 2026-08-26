"""Shared fixtures.

The manager is process-wide by design — a project compiles to one shared artifact — which is
exactly what independent test cases cannot tolerate. Every test that touches it therefore resets
it, and anything that builds gets its own directory so one test's cache cannot answer another
test's question.
"""

from __future__ import annotations

import shutil
import sys
from collections.abc import Iterator
from pathlib import Path

import pytest
from compylr import _manager

# The differential tiers share one runner, and it lives with the drivers it reads rather than in
# the package -- it is test scaffolding for a corpus, not part of compylr's surface. Putting its
# directory on the path is what lets `import _runner` work from here without making
# `python/fixtures/` a package, which would change what maturin packages.
_DRIVERS = Path(__file__).resolve().parents[1] / "fixtures" / "drivers"
if str(_DRIVERS) not in sys.path:
    sys.path.insert(0, str(_DRIVERS))


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
