"""The driver format and the shared runner.

A driver states which calls exercise a fixture. Both differential tiers read the same
declaration -- the boundary tier from Python, the translation tier by asking Python to render
it as JSON -- so the format is literal data rather than executable code. Two independent
statements of the same calls would be free to drift, and a tier exercising calls the other does
not is a tier reporting on a different program.
"""

from __future__ import annotations

import json
import math
import re
from pathlib import Path

import _runner as runner
import pytest

_ACCEPTED = Path(__file__).resolve().parents[1] / "fixtures" / "accepted"
_DRIVERS_DIR = Path(__file__).resolve().parents[1] / "fixtures" / "drivers"
_STEMS = sorted(p.stem for p in _ACCEPTED.glob("*.py"))


def write(tmp_path: Path, body: str, name: str = "sample.py") -> Path:
    path = tmp_path / name
    path.write_text(body)
    return path


class TestLoading:
    """A driver is read as data, never executed."""

    def test_a_free_function_call_names_a_member_and_its_arguments(self, tmp_path: Path) -> None:
        path = write(tmp_path, 'CALLS = [{"call": "add", "args": [2, 3]}]\n')
        assert runner.load_calls(path) == [{"call": "add", "args": [2, 3]}]

    def test_a_class_call_names_constructor_arguments_and_ordered_methods(
        self, tmp_path: Path
    ) -> None:
        path = write(
            tmp_path,
            'CALLS = [{"new": "Counter", "args": [5], "methods": [["bump", [1]], ["get", []]]}]\n',
        )
        (entry,) = runner.load_calls(path)
        assert entry["new"] == "Counter"
        assert entry["args"] == [5]
        # Order is part of the declaration: bumping after reading is a different program.
        assert entry["methods"] == [["bump", [1]], ["get", []]]

    def test_the_declaration_is_literal_data_not_executed(self, tmp_path: Path) -> None:
        # If the loader imported the module, this would raise SystemExit rather than a
        # DriverError -- which is the whole reason the format is literal.
        path = write(tmp_path, "import sys\nsys.exit(1)\nCALLS = []\n")
        with pytest.raises(runner.DriverError):
            runner.load_calls(path)

    def test_a_driver_without_calls_is_refused_by_name(self, tmp_path: Path) -> None:
        path = write(tmp_path, "OTHER = []\n", name="lonely.py")
        with pytest.raises(runner.DriverError) as caught:
            runner.load_calls(path)
        assert "lonely.py" in str(caught.value)
        assert "CALLS" in str(caught.value)

    def test_a_computed_declaration_is_refused_by_name(self, tmp_path: Path) -> None:
        path = write(tmp_path, "CALLS = [dict(call='add')]\n", name="computed.py")
        with pytest.raises(runner.DriverError) as caught:
            runner.load_calls(path)
        assert "computed.py" in str(caught.value)
        assert "literal" in str(caught.value)

    @pytest.mark.parametrize(
        ("body", "wrong"),
        [
            ('CALLS = {"call": "add"}\n', "list"),
            ("CALLS = [42]\n", "mapping"),
            ('CALLS = [{"args": [1]}]\n', "call"),
            ('CALLS = [{"call": "add", "new": "C"}]\n', "both"),
            ('CALLS = [{"call": "add", "args": 3}]\n', "list"),
            ('CALLS = [{"new": "C", "args": [], "methods": [["m"]]}]\n', "name and its arguments"),
            ('CALLS = [{"call": "add", "args": [], "extra": 1}]\n', "extra"),
        ],
    )
    def test_a_malformed_driver_names_the_driver_and_the_problem(
        self, tmp_path: Path, body: str, wrong: str
    ) -> None:
        path = write(tmp_path, body, name="broken.py")
        with pytest.raises(runner.DriverError) as caught:
            runner.load_calls(path)
        message = str(caught.value)
        assert "broken.py" in message, message
        assert wrong in message, message


class TestMembersNamed:
    """Coverage checks ask a driver which members it reaches."""

    def test_it_reports_functions_and_classes(self, tmp_path: Path) -> None:
        path = write(
            tmp_path,
            'CALLS = [{"call": "add", "args": [1, 2]},'
            ' {"new": "Counter", "args": [0], "methods": [["get", []]]}]\n',
        )
        assert runner.members_named(runner.load_calls(path)) == {"add", "Counter"}

    def test_a_mapping_argument_is_data_not_a_call(self, tmp_path: Path) -> None:
        # Fixtures take dict arguments. Reading every mapping in argument position as a call
        # would make `lookup({"a": 1}, "a")` unstatable.
        path = write(tmp_path, 'CALLS = [{"call": "lookup", "args": [{"a": 1}, "a"]}]\n')
        assert runner.members_named(runner.load_calls(path)) == {"lookup"}

    def test_it_reports_a_class_constructed_as_an_argument(self, tmp_path: Path) -> None:
        path = write(
            tmp_path,
            'CALLS = [{"call": "read", "args": [{"new": "Counter", "args": [3]}]}]\n',
        )
        assert runner.members_named(runner.load_calls(path)) == {"read", "Counter"}


class TestEncoding:
    """JSON carries a driver to the translation tier without losing a type."""

    def test_values_json_represents_exactly_are_left_alone(self) -> None:
        calls = [{"call": "f", "args": [1, 2.5, "s", True, None, [1, 2]]}]
        assert runner.encode_calls(calls) == [
            {"call": "f", "args": [1, 2.5, "s", True, None, [1, 2]], "methods": []}
        ]

    def test_a_set_stays_distinguishable_from_a_list(self) -> None:
        (entry,) = runner.encode_calls([{"call": "f", "args": [{2, 1}]}])
        # Without the tag this would arrive as `[1, 2]` and the harness would build a Vec.
        assert entry["args"] == [{"$set": [1, 2]}]

    def test_a_tuple_stays_distinguishable_from_a_list(self) -> None:
        (entry,) = runner.encode_calls([{"call": "f", "args": [(1, "a")]}])
        assert entry["args"] == [{"$tuple": [1, "a"]}]

    def test_a_mapping_keeps_its_key_type(self) -> None:
        (entry,) = runner.encode_calls([{"call": "f", "args": [{1: "a"}]}])
        assert entry["args"] == [{"$dict": [[1, "a"]]}]

    def test_a_constructed_argument_stays_a_call(self) -> None:
        (entry,) = runner.encode_calls(
            [{"call": "read", "args": [{"new": "Counter", "args": [3]}]}]
        )
        assert entry["args"] == [{"new": "Counter", "args": [3], "methods": []}]

    def test_methods_are_carried_with_their_arguments(self) -> None:
        (entry,) = runner.encode_calls(
            [{"new": "Grid", "args": [2], "methods": [["write", [0, 1, {2, 3}]]]}]
        )
        assert entry["methods"] == [["write", [0, 1, {"$set": [2, 3]}]]]

    def test_the_whole_corpus_encodes(self) -> None:
        # Every driver in the tree must survive the trip, or the translation tier cannot read it.
        drivers = Path(__file__).resolve().parents[1] / "fixtures" / "drivers"
        for path in sorted(drivers.glob("*.py")):
            if path.name.startswith("_"):
                continue
            json.dumps(runner.encode_calls(runner.load_calls(path)))


class TestRunning:
    """The runner returns values, so the boundary tier can compare objects."""

    def test_it_calls_a_free_function(self) -> None:
        module = _module(add=lambda a, b: a + b)
        results = runner.run_calls([{"call": "add", "args": [2, 3]}], module)
        assert results == [5]

    def test_it_constructs_and_calls_methods_in_order(self) -> None:
        module = _module(Counter=_Counter)
        results = runner.run_calls(
            [{"new": "Counter", "args": [5], "methods": [["bump", [1]], ["get", []]]}],
            module,
        )
        # `bump` returns None; `get` sees the bump that preceded it.
        assert results == [[None, 6]]

    def test_it_constructs_an_instance_to_pass_as_an_argument(self) -> None:
        module = _module(Counter=_Counter, read=lambda c: c.get())
        results = runner.run_calls(
            [{"call": "read", "args": [{"new": "Counter", "args": [4]}]}], module
        )
        assert results == [4]

    def test_it_calls_methods_on_a_returned_instance(self) -> None:
        module = _module(Counter=_Counter, build=lambda n: _Counter(n))
        results = runner.run_calls(
            [{"call": "build", "args": [7], "methods": [["get", []]]}], module
        )
        assert results == [[7]]

    def test_a_missing_member_is_reported(self) -> None:
        with pytest.raises(runner.DriverError) as caught:
            runner.run_calls([{"call": "absent", "args": []}], _module())
        assert "absent" in str(caught.value)


class TestTranscript:
    """One canonical rendering, produced identically by Python and by generated Rust."""

    @pytest.mark.parametrize(
        ("value", "expected"),
        [
            (3, "3"),
            (-7, "-7"),
            (True, "true"),
            (False, "false"),
            (None, "null"),
            ("hi", '"hi"'),
            ('a"b', '"a\\"b"'),
            ("héllo", '"h\\u00e9llo"'),
            ([1, 2, 3], "[1,2,3]"),
            ([], "[]"),
            ((1, "a"), '[1,"a"]'),
        ],
    )
    def test_each_type_has_one_rendering(self, value: object, expected: str) -> None:
        assert runner.render_value(value) == expected

    def test_a_mapping_renders_with_sorted_keys(self) -> None:
        # Insertion order deliberately differs from sorted order: the subset promises no
        # mapping order, so a rendering that preserved insertion would be flaky, not correct.
        assert runner.render_value({"b": 2, "a": 1}) == '{"a":1,"b":2}'
        assert runner.render_value({"b": 2, "a": 1}) == runner.render_value({"a": 1, "b": 2})

    def test_an_integer_keyed_mapping_sorts_by_key_not_by_spelling(self) -> None:
        assert runner.render_value({10: "x", 9: "y"}) == '{"9":"y","10":"x"}'

    def test_a_set_renders_as_a_sorted_array(self) -> None:
        assert runner.render_value({3, 1, 2}) == "[1,2,3]"
        assert runner.render_value({1, 2, 3}) == runner.render_value({3, 2, 1})

    def test_a_float_has_one_fixed_representation(self) -> None:
        assert runner.render_value(0.5) == "5.00000000e-1"
        assert runner.render_value(-2.0) == "-2.00000000e+0"
        assert runner.render_value(1 / 3) == "3.33333333e-1"

    def test_a_float_rendering_absorbs_a_difference_inside_the_tolerance(self) -> None:
        # The renderer's precision is what the tolerance implies, so two values that compare
        # equal under it also render identically -- which is what lets the translation tier
        # compare text and still honour D4.
        near = 1.0 + 1e-12
        assert math.isclose(near, 1.0, rel_tol=runner.FLOAT_RELATIVE_TOLERANCE)
        assert runner.render_value(near) == runner.render_value(1.0)

    def test_nested_containers_render_recursively(self) -> None:
        assert runner.render_value({"a": [1, {2, 1}]}) == '{"a":[1,[1,2]]}'

    def test_a_transcript_is_one_line_per_call(self) -> None:
        assert runner.render_transcript([1, "a", [2]]) == '1\n"a"\n[2]'

    def test_an_unrenderable_value_is_reported(self) -> None:
        with pytest.raises(runner.DriverError):
            runner.render_value(object())


class TestTolerance:
    """One constant, so the two places that compare floats cannot disagree."""

    def test_it_matches_the_tolerance_the_demo_already_uses(self) -> None:
        source = (
            Path(__file__).resolve().parents[2] / "demo/src/algorithms/__main__.py"
        ).read_text()
        spelled = re.search(r"rel_tol=([0-9eE.+-]+)", source)
        assert spelled is not None, "the demo must still compare floats with a tolerance"
        assert float(spelled.group(1)) == runner.FLOAT_RELATIVE_TOLERANCE

    def test_the_renderer_precision_is_derived_from_it(self) -> None:
        implied = round(-math.log10(runner.FLOAT_RELATIVE_TOLERANCE))
        assert implied == runner.FLOAT_SIGNIFICANT_DIGITS

    def test_values_agreeing_within_the_tolerance_compare_equal(self) -> None:
        assert runner.values_agree(1.0, 1.0 + 1e-12)
        assert not runner.values_agree(1.0, 1.0 + 1e-6)

    def test_floats_nested_in_containers_use_the_tolerance(self) -> None:
        assert runner.values_agree([1.0, {"a": 2.0}], [1.0 + 1e-12, {"a": 2.0 + 1e-12}])
        assert not runner.values_agree([1.0], [1.5])

    def test_a_mapping_compares_equal_regardless_of_iteration_order(self) -> None:
        assert runner.values_agree({"a": 1, "b": 2}, {"b": 2, "a": 1})

    def test_unequal_types_do_not_agree(self) -> None:
        assert not runner.values_agree(1, "1")
        assert not runner.values_agree([1], [1, 2])


class _Counter:
    def __init__(self, start: int) -> None:
        self.count = start

    def bump(self, by: int) -> None:
        self.count += by

    def get(self) -> int:
        return self.count


def _module(**members: object) -> object:
    """A stand-in for an imported fixture module."""
    return type("module", (), members)


class TestTheCorpusRuns:
    """Every driver actually exercises its fixture under CPython.

    A driver that produces nothing proves nothing, and one whose transcript varies between runs
    would make the differential tiers flaky rather than make the compiler wrong.
    """

    @pytest.mark.parametrize("stem", _STEMS)
    def test_a_driver_writes_at_least_one_line(self, stem: str) -> None:
        results = runner.interpreted_results(_ACCEPTED, _DRIVERS_DIR, stem)
        transcript = runner.render_transcript(results)
        assert transcript.strip(), f"{stem} produced no output"
        assert len(transcript.splitlines()) >= 1

    @pytest.mark.parametrize("stem", _STEMS)
    def test_a_driver_is_deterministic(self, stem: str) -> None:
        once = runner.render_transcript(runner.interpreted_results(_ACCEPTED, _DRIVERS_DIR, stem))
        twice = runner.render_transcript(runner.interpreted_results(_ACCEPTED, _DRIVERS_DIR, stem))
        assert once == twice, f"{stem} rendered differently on a second run"

    def test_the_cross_source_pair_resolves_as_one_namespace(self) -> None:
        # `caller` reaches a function defined in the other source. Running it alone would raise
        # NameError, which is why the grouping rule is stated once and shared by both tiers.
        assert runner.group_for("cross_source_caller", _STEMS) == [
            "cross_source_callee",
            "cross_source_caller",
        ]
        assert runner.group_for("arithmetic", _STEMS) == ["arithmetic"]
