"""The boundary tier: compiled members must answer what the same Python answers.

The translation tier already exercises this corpus through generated Rust. This one exercises it
the way a user reaches it -- across the host bridge -- and the two fail differently on purpose. A
program can be translated correctly and *converted* wrongly at the boundary, and only this tier
sees that: it is where a text argument costs about 42ns per element, and where `binary_search`
turned out sixteen times slower compiled than interpreted.

Values are compared here, not text. This tier already holds both answers as Python objects, so
`==` compares mappings and sets by content rather than by an iteration order the subset does not
promise, and floats compare within the shared tolerance. Rendering them to text first would invent
an ordering problem the comparison does not have.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import _runner as runner
import pytest
from compylr import _core
from compylr._build import BuildPipeline
from compylr._config import Behavior
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]

ACCEPTED = Path(__file__).resolve().parents[1] / "fixtures" / "accepted"
DRIVERS = Path(__file__).resolve().parents[1] / "fixtures" / "drivers"
STEMS = sorted(path.stem for path in ACCEPTED.glob("*.py"))
BOUNDARY_STEMS = STEMS


@pytest.fixture(scope="module")
def compiled(tmp_path_factory: pytest.TempPathFactory) -> SimpleNamespace:
    """The whole accepted corpus, built as ONE extension.

    One build for the corpus, not one per fixture -- which is also how a real project is built:
    every marked member in a project shares a single artifact, and it is what keeps this tier's
    cost a single build rather than eighteen.

    Each fixture is handed over as a **whole source** rather than marked member by member, keeping
    its classes and functions together exactly as whole-project compilation eventually assembles
    them. The boundary tier covers every accepted fixture without exclusions.
    """
    behavior = Behavior.from_language("python").to_core()
    sources = [((ACCEPTED / f"{stem}.py").read_text(), behavior) for stem in BOUNDARY_STEMS]

    started = time.perf_counter()
    unit = _core.compile_unit(sources, "rust")
    module = BuildPipeline(tmp_path_factory.mktemp("corpus") / ".compylr").build(unit)
    elapsed = time.perf_counter() - started
    print(f"\n[boundary tier] one build for {len(BOUNDARY_STEMS)} fixtures: {elapsed:.1f}s")

    return SimpleNamespace(**{name: getattr(module, name) for name in dir(module)})


@pytest.fixture(scope="module")
def interpreted() -> dict[str, list]:
    """What CPython answers, produced in a process where compilation is off.

    A separate process, and `COMPYLR_DISABLE=1` in it, so that nothing about the expected answer
    can depend on the compiler being correct -- including a marked member reaching another marked
    member through module globals.
    """
    script = """
import json, pathlib, sys
sys.path.insert(0, sys.argv[1])
import _runner

drivers, accepted = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
out = {}
for path in sorted(drivers.glob("*.py")):
    if path.name.startswith("_"):
        continue
    results = _runner.interpreted_results(accepted, drivers, path.stem)
    out[path.stem] = _runner.encode_value(results)
print(json.dumps(out))
"""
    environment = {**os.environ, "COMPYLR_DISABLE": "1"}
    finished = subprocess.run(
        [sys.executable, "-c", script, str(DRIVERS), str(ACCEPTED)],
        capture_output=True,
        text=True,
        env=environment,
        check=False,
    )
    assert finished.returncode == 0, f"the oracle process failed:\n{finished.stderr}"
    encoded = json.loads(finished.stdout)
    return {stem: runner.decode_value(value) for stem, value in encoded.items()}


def _describe(entry: dict) -> str:
    """Name a call the way its driver wrote it."""
    member = entry.get("call", entry.get("new"))
    arguments = ", ".join(repr(argument) for argument in entry.get("args", []))
    methods = entry.get("methods", [])
    suffix = "".join(f".{name}({', '.join(map(repr, args))})" for name, args in methods)
    prefix = "" if "call" in entry else "new "
    return f"{prefix}{member}({arguments}){suffix}"


class TestTheCorpusAgrees:
    @pytest.mark.parametrize("stem", BOUNDARY_STEMS)
    def test_compiled_answers_what_the_same_python_answers(
        self, stem: str, compiled: SimpleNamespace, interpreted: dict[str, list]
    ) -> None:
        calls = runner.load_calls(DRIVERS / f"{stem}.py")
        actual = runner.run_calls(calls, compiled)
        expected = interpreted[stem]

        assert len(actual) == len(expected), f"{stem}: the two runs made different numbers of calls"
        for entry, got, want in zip(calls, actual, expected, strict=True):
            assert runner.values_agree(got, want), (
                f"{stem}: {_describe(entry)}\n  compiled:    {got!r}\n  interpreted: {want!r}"
            )

    def test_every_fixture_was_covered(self, interpreted: dict[str, list]) -> None:
        # The parametrisation above is derived from the directory, so this guards the other end:
        # an oracle that quietly produced nothing for a fixture would make its row vacuous.
        assert set(interpreted) == set(STEMS)
        assert all(interpreted[stem] for stem in STEMS)


class TestTheComparisonIsNotTextual:
    """Guards against someone later "fixing" this tier into a text comparison.

    Values are the point here. The subset promises neither mapping nor set iteration order, so a
    text comparison would be asserting an order the language does not provide -- flaky rather
    than correct.
    """

    def test_a_mapping_compares_equal_regardless_of_iteration_order(
        self, compiled: SimpleNamespace
    ) -> None:
        counted = compiled.counts(["b", "a", "b"])
        assert runner.values_agree(counted, {"a": 1, "b": 2})
        assert runner.values_agree(counted, {"b": 2, "a": 1})
        # And the rendering of the two orders is identical, which is the property the other tier
        # leans on.
        assert runner.render_value({"a": 1, "b": 2}) == runner.render_value({"b": 2, "a": 1})

    def test_a_set_compares_equal_regardless_of_iteration_order(
        self, compiled: SimpleNamespace
    ) -> None:
        unique = compiled.unique()
        assert runner.values_agree(unique, {1, 2})
        assert runner.values_agree(unique, {2, 1})

    def test_a_float_compares_within_the_tolerance(self, compiled: SimpleNamespace) -> None:
        ratio = compiled.ratio(1, 3)
        assert runner.values_agree(ratio, 1 / 3)
        assert not runner.values_agree(ratio, 0.334)


class TestADisagreementIsReported:
    def test_the_message_names_the_call_and_both_values(self) -> None:
        entry = {"call": "modulo", "args": [-7, 3]}
        described = _describe(entry)
        assert "modulo" in described
        assert "-7" in described

    def test_a_constructed_call_is_described_with_its_methods(self) -> None:
        entry = {"new": "Counter", "args": [0], "methods": [["bump", [5]], ["get", []]]}
        described = _describe(entry)
        assert "new Counter(0)" in described
        assert ".bump(5)" in described
        assert ".get()" in described
