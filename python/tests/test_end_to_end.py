"""The whole pipeline, compiling real Rust.

These tests are slow — each build compiles a crate — so they are marked and kept few, with each one
earning its cost by checking something no faster test can. The claim they exist to support is the
only one that really matters: **a compiled function returns what the interpreted function would**,
including on the operands where Rust and Python disagree.
"""

from __future__ import annotations

import json
from pathlib import Path

import compylr
import pytest
from compylr._build import BuildPipeline
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]


# Interpreted references. Each is compiled and then compared against the original.
def _add(a: int, b: int) -> int:
    return a + b


def _floordiv(a: int, b: int) -> int:
    return a // b


def _modulo(a: int, b: int) -> int:
    return a % b


def _ratio(a: int, b: int) -> float:
    return a / b


def _concat(a: str, b: str) -> str:
    return a + b


@pytest.fixture(scope="module")
def project(tmp_path_factory: pytest.TempPathFactory) -> compylr.Manager:
    """A manager with several functions marked and built once for the whole module."""
    from compylr import _manager

    _manager._reset_for_tests()
    root = tmp_path_factory.mktemp("project") / ".compylr"
    c = compylr.initialize(root=root)

    c.compyle(_add)
    c.compyle(_floordiv)
    c.compyle(_modulo)
    c.compyle(_ratio)
    c.compyle(_concat)
    c.ensure_built()
    return c


class TestCompiledResultsMatchInterpreted:
    @pytest.mark.parametrize(
        ("a", "b"),
        [(-7, 2), (7, -2), (-7, -2), (7, 2), (-1, 5), (1, -5), (0, 3)],
    )
    def test_integer_operators_agree_on_signed_operands(
        self, project: compylr.Manager, a: int, b: int
    ) -> None:
        # The cases where Rust's native operators disagree with Python's. If the backend had
        # mapped `//` and `%` straight through, these are the rows that would fail.
        compiled = project._functions
        assert compiled["_floordiv"](a, b) == _floordiv(a, b)
        assert compiled["_modulo"](a, b) == _modulo(a, b)
        assert compiled["_ratio"](a, b) == _ratio(a, b)
        assert compiled["_add"](a, b) == _add(a, b)

    def test_string_concatenation_agrees(self, project: compylr.Manager) -> None:
        compiled = project._functions["_concat"]
        assert compiled("ab", "cd") == _concat("ab", "cd")
        assert compiled("", "") == _concat("", "")

    def test_true_division_returns_a_float(self, project: compylr.Manager) -> None:
        result = project._functions["_ratio"](6, 3)
        assert result == 2.0
        assert isinstance(result, float)


class TestFailuresBecomeExceptions:
    def test_division_by_zero(self, project: compylr.Manager) -> None:
        with pytest.raises(ZeroDivisionError):
            project._functions["_floordiv"](1, 0)
        with pytest.raises(ZeroDivisionError):
            project._functions["_ratio"](1, 0)

    def test_overflow(self, project: compylr.Manager) -> None:
        with pytest.raises(OverflowError):
            project._functions["_add"](2**63 - 1, 1)

    def test_wrong_argument_type(self, project: compylr.Manager) -> None:
        with pytest.raises(TypeError):
            project._functions["_add"]("x", 1)

    def test_the_process_survives(self, project: compylr.Manager) -> None:
        with pytest.raises(ZeroDivisionError):
            project._functions["_floordiv"](1, 0)
        assert project._functions["_add"](40, 2) == 42


class TestArtifacts:
    def test_the_ir_is_written_and_readable(self, project: compylr.Manager) -> None:
        artifact = json.loads(project.paths.ir.read_text())
        names = {f["name"] for f in artifact["functions"]}
        assert {"_add", "_floordiv", "_modulo", "_ratio", "_concat"} <= names

    def test_the_generated_rust_is_written(self, project: compylr.Manager) -> None:
        source = project.paths.target_source.read_text()
        assert "pub mod generated" in source
        assert "py_floordiv" in source, "the semantics-preserving helper must be in the output"

    def test_all_generated_files_share_one_root(self, project: compylr.Manager) -> None:
        root = project.paths.root
        for path in (project.paths.ir, project.paths.target_source, project.paths.state):
            assert root in path.parents

    def test_one_build_covers_every_function(self, project: compylr.Manager) -> None:
        state = json.loads(project.paths.state.read_text())
        assert set(state["functions"]) == {
            "_add",
            "_floordiv",
            "_modulo",
            "_ratio",
            "_concat",
        }


class TestRebuildCache:
    def test_an_unchanged_project_does_not_rebuild(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        def fail(*args: object, **kwargs: object) -> None:
            raise AssertionError("an unchanged project must not invoke the toolchain")

        monkeypatch.setattr(BuildPipeline, "build", fail)
        project.ensure_built()

    def test_repeated_calls_do_not_rebuild(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        def fail(*args: object, **kwargs: object) -> None:
            raise AssertionError("calling must not rebuild")

        monkeypatch.setattr(BuildPipeline, "build", fail)
        for _ in range(5):
            assert project._functions["_add"](1, 1) == 2

    def test_a_failed_build_is_not_recorded_as_successful(self, tmp_path: Path) -> None:
        from compylr import _manager

        _manager._reset_for_tests()
        c = compylr.initialize(root=tmp_path / ".compylr")

        def broken(a: int) -> int:
            return a

        c.compyle(broken)
        # Corrupt the emitted source so the toolchain fails, then confirm nothing was recorded.
        original = BuildPipeline.write_artifacts

        def sabotage(self: BuildPipeline, compiled: object) -> None:
            original(self, compiled)  # type: ignore[arg-type]
            self.paths.target_source.write_text("this is not valid rust")

        BuildPipeline.write_artifacts = sabotage  # type: ignore[method-assign]
        try:
            with pytest.raises(compylr.BuildError) as caught:
                c.ensure_built()
        finally:
            BuildPipeline.write_artifacts = original  # type: ignore[method-assign]

        assert "error" in str(caught.value).lower(), "the toolchain's diagnostics must come through"
        assert not c.paths.state.exists(), "a failed build must not be cached as a success"


class TestReuseAcrossProcesses:
    def test_a_second_manager_reuses_the_built_artifact(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # Stands in for a later run: same artifact directory, fresh manager, no toolchain allowed.
        from compylr import _manager

        root = project.paths.root
        _manager._reset_for_tests()

        def fail(*args: object, **kwargs: object) -> None:
            raise AssertionError("a later run must reuse the artifact rather than rebuild")

        monkeypatch.setattr(BuildPipeline, "build", fail)

        fresh = compylr.initialize(root=root)
        fresh.compyle(_add)
        fresh.compyle(_floordiv)
        fresh.compyle(_modulo)
        fresh.compyle(_ratio)
        fresh.compyle(_concat)

        assert fresh._functions["_floordiv"](-7, 2) == -4
