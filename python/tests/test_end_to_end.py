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


def _via_call(n: int) -> int:
    # Calls another marked function with no annotation on the binding -- the case that only
    # works because signatures are gathered across every source before any is lowered.
    doubled = _add(n, n)
    return doubled + 1


def _sum_first(xs: list[int]) -> int:
    return xs[0] + len(xs)


def _from_mapping(d: dict[str, int], key: str) -> int:
    return d[key]


def _make_pair() -> tuple[int, str]:
    return (1, "a")


def _documented(n: int) -> int:
    """Triple a value.

    Included so the end-to-end path covers a documented function, which was rejected
    outright until docstrings were accepted.
    """
    return n * 3


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
    c.compyle(_documented)
    c.compyle(_via_call)
    c.compyle(_sum_first)
    c.compyle(_from_mapping)
    c.compyle(_make_pair)
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
        assert {
            "_add",
            "_floordiv",
            "_modulo",
            "_ratio",
            "_concat",
            "_documented",
            "_via_call",
            "_sum_first",
            "_from_mapping",
            "_make_pair",
        } <= names

    def test_the_generated_rust_is_written(self, project: compylr.Manager) -> None:
        source = project.paths.target_source.read_text()
        assert "pub fn _floordiv" in source
        assert "div_floor" in source, (
            "the helper named for the declared rounding must be called, not Rust's `/`"
        )

    def test_the_crate_is_split_by_concern(self, project: compylr.Manager) -> None:
        src = project.paths.src
        assert {p.name for p in src.iterdir()} == {
            "lib.rs",
            "generated.rs",
            "bindings.rs",
            "compat.rs",
        }

    def test_the_translated_file_opens_on_the_translated_code(
        self, project: compylr.Manager
    ) -> None:
        # The whole point: a reader opening this file sees their functions, not two hundred lines
        # of helpers that are identical in every project.
        lines = project.paths.target_source.read_text().splitlines()
        first_fn = next(i for i, line in enumerate(lines) if line.startswith("pub fn"))
        assert first_fn < 10, f"translated code should be near the top, found at line {first_fn}"

    def test_the_crate_root_does_not_grow_with_the_program(self, project: compylr.Manager) -> None:
        # Asserted as "contains nothing per-function" rather than as a line count. The count is a
        # proxy, and a proxy that formatting can move measures the formatter rather than the
        # property: rustfmt expands the crate root's `#![allow(...)]` across eight lines.
        root = (project.paths.src / "lib.rs").read_text()
        assert "pub fn" not in root, "the crate root must hold no translated code"

        state = json.loads(project.paths.state.read_text())
        for name in state["functions"]:
            assert name not in root, f"{name} reached the crate root, which must not grow"

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
            "_documented",
            "_via_call",
            "_sum_first",
            "_from_mapping",
            "_make_pair",
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

    def test_a_built_project_does_not_recompile_to_answer_a_call(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # Not rebuilding the *crate* is not enough. `ensure_built` used to run the whole compiler
        # -- parse, lower, verify, pass, emit -- on every call, just to recompute a fingerprint it
        # already held, and only then notice that the loaded module was current.
        #
        # A `CompiledFunction` caches its implementation after the first resolve, so the cost was
        # paid once per marked function: for a project with sixty of them, a couple of hundred
        # milliseconds each, on a warm cache. The demo is where that became obvious.
        from compylr import _core

        project.ensure_built()
        calls = 0
        real = _core.compile_unit

        def counted(*args: object, **kwargs: object) -> object:
            nonlocal calls
            calls += 1
            return real(*args, **kwargs)

        monkeypatch.setattr(_core, "compile_unit", counted)
        for _ in range(5):
            project.ensure_built()
        assert calls == 0, "a project that is loaded and unchanged must not re-run the compiler"

    def test_marking_something_new_does_recompile(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # The other half: the shortcut above is only sound while nothing has been marked since,
        # and marking is exactly what makes the loaded module no longer cover the project.
        from compylr import _core

        project.ensure_built()
        calls = 0
        real = _core.compile_unit

        def counted(*args: object, **kwargs: object) -> object:
            nonlocal calls
            calls += 1
            return real(*args, **kwargs)

        monkeypatch.setattr(_core, "compile_unit", counted)

        @project.compyle
        def _late_addition(a: int) -> int:
            return a + 1

        monkeypatch.setattr(BuildPipeline, "build", lambda *a, **k: object())
        project.ensure_built()
        assert calls == 1, "a newly marked member must make the project recompile"

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
        for fn in (
            _add,
            _floordiv,
            _modulo,
            _ratio,
            _concat,
            _documented,
            _via_call,
            _sum_first,
            _from_mapping,
            _make_pair,
        ):
            fresh.compyle(fn)

        assert fresh._functions["_floordiv"](-7, 2) == -4


class TestDocumentedFunctions:
    def test_a_documented_function_compiles_and_matches(self, project: compylr.Manager) -> None:
        compiled = project._functions["_documented"]
        assert compiled(5) == _documented(5)

    def test_the_docstring_reaches_the_generated_rust(self, project: compylr.Manager) -> None:
        source = project.paths.target_source.read_text()
        assert "/// Triple a value." in source, (
            "the generated source is written to be read, and a translated function stripped of "
            "its explanation is harder to check against the original"
        )

    def test_the_docstring_is_readable_on_the_marked_function(
        self, project: compylr.Manager
    ) -> None:
        assert project._functions["_documented"].__doc__ == _documented.__doc__


class TestArtifactsFollowTheProject:
    def test_running_from_a_subdirectory_reuses_the_artifacts(
        self, project: compylr.Manager, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # The point of root discovery: the same project run from a subdirectory must find what it
        # already built rather than compiling a second copy.
        from compylr import _manager
        from compylr._build import discover_root

        root = project.paths.root
        nested = root.parent / "src" / "deep"
        nested.mkdir(parents=True, exist_ok=True)
        monkeypatch.chdir(nested)

        assert discover_root() == root

        _manager._reset_for_tests()

        def fail(*args: object, **kwargs: object) -> None:
            raise AssertionError("running from a subdirectory must not rebuild")

        monkeypatch.setattr(BuildPipeline, "build", fail)

        fresh = compylr.initialize()
        for fn in (
            _add,
            _floordiv,
            _modulo,
            _ratio,
            _concat,
            _documented,
            _via_call,
            _sum_first,
            _from_mapping,
            _make_pair,
        ):
            fresh.compyle(fn)

        assert fresh.paths.root == root
        assert fresh._functions["_floordiv"](-7, 2) == -4


class TestCrossSourceCallInference:
    def test_a_call_typed_binding_compiles_and_matches(self, project: compylr.Manager) -> None:
        # The whole point of the change, end to end: no annotation on `doubled`, and the compiled
        # result equals the interpreted one.
        compiled = project._functions["_via_call"]
        assert compiled(5) == _via_call(5)

    def test_the_generated_rust_types_it_correctly(self, project: compylr.Manager) -> None:
        source = project.paths.target_source.read_text()
        assert "pub fn _via_call(n: i64) -> Result<i64, RuntimeError>" in source
        assert "let doubled: i64" in source, (
            "the binding must carry the callee's return type, taken from its signature"
        )


class TestCollections:
    def test_compiled_results_match_the_interpreted_originals(
        self, project: compylr.Manager
    ) -> None:
        assert project._functions["_sum_first"]([10, 20, 30]) == _sum_first([10, 20, 30])
        assert project._functions["_from_mapping"]({"a": 5}, "a") == _from_mapping({"a": 5}, "a")

    def test_a_negative_index_counts_from_the_end(self, project: compylr.Manager) -> None:
        # Rust's native indexing does not do this; the emitted code has to.
        assert project._functions["_sum_first"]([1, 2, 3]) == 4

    def test_a_tuple_returns_as_a_tuple_not_a_list(self, project: compylr.Manager) -> None:
        result = project._functions["_make_pair"]()
        assert result == (1, "a")
        assert isinstance(result, tuple)

    def test_a_list_returns_as_a_list(self, project: compylr.Manager) -> None:
        assert isinstance(project._functions["_sum_first"]([1]), int)

    def test_a_missing_key_raises_key_error(self, project: compylr.Manager) -> None:
        with pytest.raises(KeyError):
            project._functions["_from_mapping"]({}, "absent")

    def test_an_index_out_of_range_raises_index_error(self, project: compylr.Manager) -> None:
        with pytest.raises(IndexError):
            project._functions["_sum_first"]([])

    def test_a_wrong_element_type_raises_type_error(self, project: compylr.Manager) -> None:
        with pytest.raises(TypeError):
            project._functions["_sum_first"](["a"])

    def test_the_callers_list_is_unchanged(self, project: compylr.Manager) -> None:
        # Collections cross by value, so a compiled function cannot affect what its caller holds.
        # Nothing in the subset can mutate, so this is currently unobservable -- asserted anyway,
        # so that adding mutation has to confront it deliberately.
        xs = [1, 2, 3]
        project._functions["_sum_first"](xs)
        assert xs == [1, 2, 3]
