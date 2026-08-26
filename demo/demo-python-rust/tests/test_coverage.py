"""The demo's claim to exercise the whole subset, checked against the IR it actually produced.

"Showcases everything compylr can do" is the kind of sentence that is true when it is written and
quietly false a release later — someone simplifies an algorithm, the last `set` literal in the
project goes away, and the sentence is still there. So it is an assertion instead.

`compylr` writes the IR of every build to `.compylr/ir/unit.json`. These read that file and check
that every statement form, expression form, type, and operator appears somewhere. When one stops
being covered, this fails and names it.

The other half of the guarantee lives in the compiler's own suite: `tests/demo_coverage.rs` reads
the IR's enum definitions and fails when a form is added that the tables here do not know about.
Without it these tests would keep passing while the thing they measure grew.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from algorithms import ir_coverage
from algorithms._compylr import c
from algorithms.ir_coverage import MODES, TABLES, Coverage


@pytest.fixture(scope="module")
def coverage() -> Coverage:
    """Coverage of the artifact the build wrote.

    `ensure_built` first, because the artifact is written by the build and a test run that never
    called a compiled function would be reading whatever was on disk from last time.
    """
    c.ensure_built()
    return ir_coverage.measure(Path(c.paths.ir))


class TestTheWholeSubsetIsExercised:
    @pytest.mark.parametrize("table", sorted(TABLES))
    def test_every_form_appears(self, table: str, coverage: Coverage) -> None:
        missing = coverage.missing(table)
        assert not missing, (
            f"the demo no longer exercises {table}: {', '.join(missing)}. "
            f"Add an algorithm that uses them, or say in the README that the claim has narrowed."
        )

    def test_both_division_modes_appear(self, coverage: Coverage) -> None:
        # `Div` is one variant carrying a mode, so covering the variant is not covering both
        # readings: Python's `/` is exact and its `//` rounds toward negative infinity.
        assert coverage.modes == MODES

    def test_the_report_says_so_and_names_where(self, coverage: Coverage) -> None:
        assert coverage.gaps() == {}
        report = coverage.report()
        for table, forms in TABLES.items():
            assert f"{table} — {len(forms)}/{len(forms)}" in report
        assert "MISSING" not in report

    def test_each_form_is_attributed_to_a_member_that_exists(self, coverage: Coverage) -> None:
        # The report names the first member to reach each form. A name that is not in the unit
        # would mean the walk is reading something other than what it claims to.
        artifact = json.loads(Path(c.paths.ir).read_text())
        members = {f["name"] for f in artifact["functions"]} | {
            k["name"] for k in artifact["classes"]
        }
        for table, forms in TABLES.items():
            for form in forms:
                assert coverage.first_use[form] in members, (table, form)


class TestTheMeasurementItself:
    """A coverage check that cannot fail is worse than none, so it is checked by deletion."""

    def test_an_empty_unit_is_reported_as_covering_nothing(self) -> None:
        empty = ir_coverage.Coverage({"functions": [], "classes": []})
        for table, forms in TABLES.items():
            assert empty.missing(table) == forms
        assert empty.gaps()
        assert "MISSING" in empty.report()

    def test_a_form_present_only_in_a_class_is_still_found(self) -> None:
        # Classes are a separate list in the artifact. A walk over `functions` alone would report
        # `SetAttr` and `Construct` missing while the demo used them constantly.
        found = ir_coverage.Coverage(
            {"functions": [], "classes": [{"name": "K", "init": {"body": [{"SetAttr": {}}]}}]}
        )
        assert found.first_use["SetAttr"] == "K"

    def test_a_variant_carrying_no_data_is_found_as_a_bare_string(self) -> None:
        # `Break` and `Continue` serialise as strings rather than as objects, so a walk that only
        # looked at dictionary keys would report both missing forever.
        found = ir_coverage.Coverage(
            {"functions": [{"name": "f", "body": ["Break", {"If": {"then": ["Continue"]}}]}]}
        )
        assert found.first_use["Break"] == "f"
        assert found.first_use["Continue"] == "f"

    def test_the_tables_have_no_duplicates(self) -> None:
        for table, forms in TABLES.items():
            assert len(set(forms)) == len(forms), table
