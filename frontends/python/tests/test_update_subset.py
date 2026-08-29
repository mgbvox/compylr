"""The generated subset matrix.

What the README claims compylr accepts is counted from the corpus rather than remembered beside
it. The property worth having is the one `py2many`'s `LANGUAGES.md` gets right: a construct is
reported as accepted **only because a fixture exercising it translated, built, ran, and agreed
with CPython**. A construct with no passing fixture does not appear.

These cover the generator's pure core -- the part that turns coverage plus evidence into a table
-- and its marker handling. What produces the coverage is a cargo invocation, tested by using it.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))

import update_subset  # noqa: E402
from _regions import MarkerError, Region, find_region, replace_region  # noqa: E402


class TestTheMatrixRestsOnEvidence:
    def test_a_construct_appears_only_when_its_fixture_agreed(self) -> None:
        tags = {"arithmetic": {"Add", "Int"}, "loops": {"While", "Int"}}
        body = update_subset.matrix_body(tags, agreed={"arithmetic"})
        assert "Add" in body
        # `While` is exercised only by a fixture that did not agree, so it is not claimed.
        assert "While" not in body

    def test_a_construct_no_fixture_exercises_does_not_appear(self) -> None:
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed={"arithmetic"})
        assert "Add" in body
        assert "Subscript" not in body

    def test_nothing_is_claimed_when_nothing_agreed(self) -> None:
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed=set())
        assert "Add" not in body

    def test_a_construct_names_the_fixture_that_exercises_it(self) -> None:
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed={"arithmetic"})
        assert "arithmetic" in body

    def test_a_construct_reached_by_several_fixtures_names_one_deterministically(self) -> None:
        tags = {"zebra": {"Add"}, "alpha": {"Add"}}
        first = update_subset.matrix_body(tags, agreed={"zebra", "alpha"})
        second = update_subset.matrix_body(
            dict(reversed(list(tags.items()))), agreed={"zebra", "alpha"}
        )
        assert first == second
        assert "alpha" in first

    def test_only_known_ir_forms_are_reported(self) -> None:
        # The artifact's tags include field names and other noise; only forms the IR actually
        # declares belong in a table describing the subset.
        body = update_subset.matrix_body(
            {"arithmetic": {"Add", "not_an_ir_form"}}, agreed={"arithmetic"}
        )
        assert "not_an_ir_form" not in body


class TestRegeneration:
    def test_it_is_idempotent(self) -> None:
        tags = {"arithmetic": {"Add", "Int"}}
        once = update_subset.matrix_body(tags, agreed={"arithmetic"})
        twice = update_subset.matrix_body(tags, agreed={"arithmetic"})
        assert once == twice

    def test_rewriting_a_region_twice_changes_nothing_the_second_time(self, tmp_path: Path) -> None:
        region = Region("matrix", tmp_path / "doc.md", prefix="subset")
        region.path.write_text(f"before\n{region.opening}\nstale\n{region.closing}\nafter\n")
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed={"arithmetic"})

        first = replace_region(region.path.read_text(), region, body)
        second = replace_region(first, region, body)
        assert first == second
        assert "stale" not in first
        assert "after" in first


class TestCheckMode:
    def test_it_fails_on_drift_and_names_what_differs(self, tmp_path: Path) -> None:
        region = Region("matrix", tmp_path / "doc.md", prefix="subset")
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed={"arithmetic"})
        region.path.write_text(f"{region.opening}\nsomething else entirely\n{region.closing}\n")

        difference = update_subset.compare(region, body)
        assert difference is not None
        assert "matrix" in difference
        assert "doc.md" in difference

    def test_it_passes_when_the_published_text_matches(self, tmp_path: Path) -> None:
        region = Region("matrix", tmp_path / "doc.md", prefix="subset")
        body = update_subset.matrix_body({"arithmetic": {"Add"}}, agreed={"arithmetic"})
        region.path.write_text(
            replace_region(f"{region.opening}\n\n{region.closing}\n", region, body)
        )

        assert update_subset.compare(region, body) is None

    def test_a_missing_marker_is_an_error_rather_than_a_silent_skip(self, tmp_path: Path) -> None:
        region = Region("matrix", tmp_path / "doc.md", prefix="subset")
        region.path.write_text("no markers here\n")
        with pytest.raises(MarkerError):
            find_region(region.path.read_text(), region)


class TestTheRealReadme:
    def test_the_readme_carries_the_region(self) -> None:
        region = update_subset.MATRIX
        assert region.path.exists()
        find_region(region.path.read_text(), region)

    def test_the_published_matrix_is_current(self) -> None:
        # The same thing `--check` does, run as part of the ordinary suite so drift is caught
        # without waiting for a hook.
        body = update_subset.generate()
        assert update_subset.compare(update_subset.MATRIX, body) is None, (
            "README's subset matrix is stale; run ./scripts/update_subset.py"
        )


class TestMarkersMode:
    """The fast half, for a commit hook.

    Regenerating the matrix runs the compiler over the whole corpus. That belongs in CI and in
    `make check`, not on a commit -- the same split `update_benchmarks.py` already makes. Moving
    or renaming a marker is what breaks the generator, and this catches that in a fraction of a
    second.
    """

    def test_it_passes_on_the_real_readme(self, capsys: pytest.CaptureFixture[str]) -> None:
        assert update_subset.main(["--markers"]) == 0
        assert "matrix" in capsys.readouterr().out

    def test_it_runs_no_subprocess(self, monkeypatch: pytest.MonkeyPatch) -> None:
        # The whole point: a commit hook must not shell out to cargo.
        def forbidden(*args: object, **kwargs: object) -> None:
            raise AssertionError("--markers must not run a subprocess")

        monkeypatch.setattr(update_subset.subprocess, "run", forbidden)
        assert update_subset.main(["--markers"]) == 0

    def test_it_fails_when_the_region_is_gone(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        missing = tmp_path / "doc.md"
        missing.write_text("no markers here\n")
        monkeypatch.setattr(update_subset, "MATRIX", Region("matrix", missing, prefix="subset"))
        assert update_subset.main(["--markers"]) == 1
