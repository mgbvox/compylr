"""The demo project, built and exercised by this repository's own suite.

A demo that stops compiling and nobody notices is worse than no demo, so it is checked here rather
than only by its own tests. Three things are asserted, and each has been wrong at some point:

* that every module of it **imports** — the precompiler reported two failures for as long as the
  command existed, and nothing was looking;
* that the answers are **genuinely compiled** — a silent fallback to interpreted Python would pass
  every agreement check in the demo and demonstrate nothing;
* that it still exercises the **whole subset** — the demo's claim to showcase compylr is a
  coverage assertion over the IR, and `tests/demo_coverage.rs` keeps the tables it measures
  against from going stale.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]

DEMO = Path(__file__).resolve().parents[2] / "demo"


@pytest.fixture(scope="module")
def demo_env() -> dict[str, str]:
    """An environment that can import the demo.

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
        timeout=1800,
    )
    assert result.returncode == 0, (
        f"the demo failed:\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
    return result.stdout


class TestPrecompilingTheDemo:
    """Precompiling the demo must import all of it.

    The demo is the only real package this repository builds, so it is the only place a package's
    `__init__.py` gets exercised end to end — and since `nth_prime` became a subpackage, the only
    place a *nested* one does.
    """

    def test_precompiling_reports_no_import_failures(self, demo_env: dict[str, str]) -> None:
        report = run_in_demo(
            demo_env,
            "from compylr import _precompile;"
            f" r = _precompile.precompile({str(DEMO / 'src')!r});"
            " print(len(r.failures));"
            " print([f.module for f in r.failures]);"
            " print(r.modules_imported)",
        ).splitlines()

        assert report[0] == "0", f"a module of the demo did not import: {report[1]}"
        # Every file under `src/`, `__init__.py` and `__main__.py` included, at every depth.
        assert int(report[2]) == len(list((DEMO / "src").rglob("*.py")))


class TestTheDemoWorks:
    def test_every_algorithm_agrees_with_its_interpreted_oracle(
        self, demo_env: dict[str, str]
    ) -> None:
        # The demo's entry point runs each algorithm and checks it against an oracle from the
        # standard library or a differently-shaped reference, then reports its exit status.
        result = subprocess.run(
            [sys.executable, "-m", "algorithms"],
            cwd=DEMO,
            env=demo_env,
            capture_output=True,
            text=True,
            timeout=1800,
        )
        assert result.returncode == 0, result.stdout + result.stderr
        assert "Every IR form a Python program can produce is exercised" in result.stdout
        assert "DISAGREES" not in result.stdout

    def test_the_three_nth_prime_variants_agree_with_the_reference(
        self, demo_env: dict[str, str]
    ) -> None:
        out = run_in_demo(
            demo_env,
            "from algorithms.nth_prime import iterative, memoized, recursive, reference\n"
            "cache = memoized.PrimeCache()\n"
            "for n in range(1, 40):\n"
            "    expected = reference.nth_prime(n)\n"
            "    assert recursive.nth_prime(n) == expected, n\n"
            "    assert iterative.nth_prime(n) == expected, n\n"
            "    assert cache.nth(n) == expected, n\n"
            "print('agree')\n",
        )
        assert "agree" in out

    def test_the_whole_demo_is_genuinely_compiled(self, demo_env: dict[str, str]) -> None:
        # The check the whole demo rests on. Without it, a fallback to interpreted Python would
        # leave every other assertion here passing.
        out = run_in_demo(
            demo_env,
            "import algorithms\n"
            "from algorithms._compylr import c\n"
            "module = c.ensure_built()\n"
            "names = ('merge_sort', 'sieve', 'UnionFind', 'recursive_nth_prime')\n"
            "for name in (*names, 'PrimeCache'):\n"
            "    assert hasattr(module, name), name\n"
            "print(module.__name__)\n",
        )
        assert out.strip().startswith("compylr_generated_"), (
            f"the demo must come from a compiled extension, got {out.strip()!r}"
        )

    def test_one_artifact_covers_both_halves_of_the_demo(self, demo_env: dict[str, str]) -> None:
        # Breadth and depth share one manager, so they share one crate. Two would mean two builds
        # and compiled functions in one that could not call the other.
        out = run_in_demo(
            demo_env,
            "import algorithms\n"
            "from algorithms._compylr import c\n"
            "print(len(c._sources))\n"
            "print(c.ensure_built() is c.ensure_built())\n",
        ).split()
        assert int(out[0]) > 50, f"the demo should mark the whole subset, found {out[0]}"
        assert out[1] == "True"

    def test_the_cache_is_actually_consulted(self, demo_env: dict[str, str]) -> None:
        out = run_in_demo(
            demo_env,
            "from algorithms.nth_prime import memoized\n"
            "cache = memoized.PrimeCache()\n"
            "cache.nth(40); cache.nth(40)\n"
            "print(cache.hit_count(), cache.known_count())\n",
        )
        assert out.split() == ["1", "1"]

    def test_the_nth_prime_entry_point_runs_and_reports_agreement(
        self, demo_env: dict[str, str]
    ) -> None:
        result = subprocess.run(
            [sys.executable, "-m", "algorithms.nth_prime", "25"],
            cwd=DEMO,
            env=demo_env,
            capture_output=True,
            text=True,
            timeout=1800,
        )
        assert result.returncode == 0, result.stderr
        assert "all four agree" in result.stdout


class TestTheDemoExercisesTheWholeSubset:
    """The demo's headline claim, checked here as well as in its own suite.

    Its own suite could be skipped or its environment could be stale; this runs the same walk over
    the artifact the build just wrote.
    """

    def test_no_ir_form_is_left_uncovered(self, demo_env: dict[str, str]) -> None:
        out = run_in_demo(
            demo_env,
            "import algorithms, json\n"
            "from algorithms import ir_coverage\n"
            "from algorithms._compylr import c\n"
            "c.ensure_built()\n"
            "print(json.dumps(ir_coverage.measure(c.paths.ir).gaps()))\n",
        )
        gaps = json.loads(out)
        assert gaps == {}, f"the demo no longer exercises: {gaps}"


class TestTheReadmeDoesNotDrift:
    def test_every_claimed_module_exists(self) -> None:
        # The README names each module. If one is renamed or removed, this fails rather than the
        # README quietly describing a project that no longer exists.
        readme = (DEMO / "README.md").read_text()
        package = DEMO / "src" / "algorithms"
        for module in (
            "sorting.py",
            "arithmetic.py",
            "stats.py",
            "text.py",
            "graphs.py",
            "dynamic.py",
            "matrices.py",
            "structures.py",
        ):
            assert module in readme, f"the README should describe {module}"
            assert (package / module).is_file(), f"{module} is missing"
        for module in ("recursive.py", "iterative.py", "memoized.py", "reference.py"):
            assert module in readme, f"the README should describe {module}"
            assert (package / "nth_prime" / module).is_file(), f"{module} is missing"

    def test_the_recursion_bound_is_stated(self) -> None:
        # It is a process abort with no traceback, which a user must not meet by surprise.
        readme = (DEMO / "README.md").read_text()
        assert "SIGSEGV" in readme
        assert "150,000" in readme

    def test_the_name_collision_finding_is_recorded(self) -> None:
        readme = (DEMO / "README.md").read_text()
        assert "shared across a whole project" in readme

    def test_the_coverage_claim_is_stated_as_checked(self) -> None:
        # The claim is the reason the breadth half exists, and it is only worth making because it
        # is an assertion. The README has to say which test enforces it.
        readme = (DEMO / "README.md").read_text()
        assert "ir_coverage" in readme
        assert "tests/test_coverage.py" in readme
