"""The demo project, built and exercised by this repository's own suite.

A demo that stops compiling and nobody notices is worse than no demo, so it is checked here rather
than only by its own tests. The assertions that matter are that all three variants agree and that
each is **genuinely compiled** — a demo silently running interpreted Python would pass every
agreement check and demonstrate nothing.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]

DEMO = Path(__file__).resolve().parents[2] / "demo"


@pytest.fixture(scope="module")
def demo_env(tmp_path_factory: pytest.TempPathFactory) -> dict[str, str]:
    """An environment that can import the demo, with its artifacts in a scratch directory.

    One build shared across every assertion in this module: the demo compiles into a single
    extension, so paying for it once is both faster and a check that it really does.
    """
    import os

    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join(
        [str(DEMO / "src"), str(Path(__file__).resolve().parents[1])]
    )
    return env


def run_in_demo(env: dict[str, str], code: str) -> str:
    """Run `code` with the demo importable, returning its stdout."""
    result = subprocess.run(
        [sys.executable, "-c", code],
        cwd=DEMO,
        env=env,
        capture_output=True,
        text=True,
        timeout=900,
    )
    assert result.returncode == 0, (
        f"the demo failed:\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
    return result.stdout


class TestTheDemoWorks:
    def test_all_three_variants_agree_with_the_reference(self, demo_env: dict[str, str]) -> None:
        out = run_in_demo(
            demo_env,
            "from nth_prime import iterative, memoized, recursive, reference\n"
            "cache = memoized.PrimeCache()\n"
            "for n in range(1, 40):\n"
            "    expected = reference.nth_prime(n)\n"
            "    assert recursive.nth_prime(n) == expected, n\n"
            "    assert iterative.nth_prime(n) == expected, n\n"
            "    assert cache.nth(n) == expected, n\n"
            "print('agree')\n",
        )
        assert "agree" in out

    def test_each_variant_is_genuinely_compiled(self, demo_env: dict[str, str]) -> None:
        # The check the whole demo rests on. Without it, a fallback to interpreted Python would
        # leave every other assertion here passing.
        out = run_in_demo(
            demo_env,
            "from nth_prime._compylr import c\n"
            "import nth_prime\n"
            "module = c.ensure_built()\n"
            "for name in ('recursive_nth_prime', 'iterative_nth_prime', 'PrimeCache'):\n"
            "    assert hasattr(module, name), name\n"
            "print(module.__name__)\n",
        )
        assert out.strip().startswith("compylr_generated_"), (
            f"the variants must come from a compiled extension, got {out.strip()!r}"
        )

    def test_the_cache_is_actually_consulted(self, demo_env: dict[str, str]) -> None:
        out = run_in_demo(
            demo_env,
            "from nth_prime import memoized\n"
            "cache = memoized.PrimeCache()\n"
            "cache.nth(40); cache.nth(40)\n"
            "print(cache.hit_count(), cache.known_count())\n",
        )
        assert out.split() == ["1", "1"]

    def test_the_entry_point_runs_and_reports_agreement(self, demo_env: dict[str, str]) -> None:
        result = subprocess.run(
            [sys.executable, "-m", "nth_prime", "25"],
            cwd=DEMO,
            env=demo_env,
            capture_output=True,
            text=True,
            timeout=900,
        )
        assert result.returncode == 0, result.stderr
        assert "all four agree" in result.stdout


class TestTheReadmeDoesNotDrift:
    def test_every_claimed_variant_exists(self) -> None:
        # The README names three variants and a reference. If one is renamed or removed, this fails
        # rather than the README quietly describing a project that no longer exists.
        readme = (DEMO / "README.md").read_text()
        for module in ("recursive.py", "iterative.py", "memoized.py", "reference.py"):
            assert module in readme, f"the README should describe {module}"
            assert (DEMO / "src" / "nth_prime" / module).is_file(), f"{module} is missing"

    def test_the_recursion_bound_is_stated(self) -> None:
        # It is a process abort with no traceback, which a user must not meet by surprise.
        readme = (DEMO / "README.md").read_text()
        assert "SIGSEGV" in readme
        assert "150,000" in readme

    def test_the_name_collision_finding_is_recorded(self) -> None:
        readme = (DEMO / "README.md").read_text()
        assert "shared across a whole project" in readme
