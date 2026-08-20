"""The native frontend, as Python sees it.

`compylr._core` is where Python hands source text to the Rust parser and lowering pass. These tests
are about that boundary: what compiles, what does not, and whether a failure arrives as something a
Python caller can actually act on.
"""

from __future__ import annotations

import json

import pytest
from compylr import _core

ADD = "def add(a: int, b: int) -> int:\n    return a + b\n"


class TestCompileUnit:
    def test_compiles_a_single_source(self) -> None:
        compiled = _core.compile_unit([ADD])

        assert compiled.function_names == ["add"]
        assert set(compiled.target_sources) == {
            "src/lib.rs",
            "src/generated.rs",
            "src/bindings.rs",
            "src/compat.rs",
        }
        assert "pub fn add" in compiled.target_sources["src/generated.rs"]
        assert "#[pymodule]" in compiled.target_sources["src/lib.rs"]
        assert all(not p.startswith("/") for p in compiled.target_sources), (
            "paths must be relative, so the caller chooses where the crate lands"
        )
        assert compiled.module_name.startswith("compylr_generated_")
        assert "pyo3" in compiled.manifest

    def test_defaults_to_the_rust_backend(self) -> None:
        assert (
            _core.compile_unit([ADD]).target_sources
            == _core.compile_unit([ADD], "rust").target_sources
        )

    def test_an_empty_project_is_not_an_error(self) -> None:
        # A project can legitimately have nothing marked yet.
        compiled = _core.compile_unit([])
        assert compiled.function_names == []

    def test_source_needs_no_file_behind_it(self) -> None:
        # This is what inspect.getsource hands back: text, with no path.
        source = "def double(n: int) -> int:\n    return n * 2\n"
        assert _core.compile_unit([source]).function_names == ["double"]

    def test_sources_are_assembled_into_one_unit(self) -> None:
        caller = "def caller(a: int) -> int:\n    return callee(a)\n"
        callee = "def callee(a: int) -> int:\n    return a * 2\n"

        forward = _core.compile_unit([caller, callee])
        backward = _core.compile_unit([callee, caller])

        assert forward.function_names == ["callee", "caller"]
        assert forward.fingerprint == backward.fingerprint

    def test_duplicate_names_are_rejected(self) -> None:
        with pytest.raises(_core.UnsupportedProgramError, match="add"):
            _core.compile_unit([ADD, ADD])

    def test_an_unresolved_call_is_rejected(self) -> None:
        with pytest.raises(_core.UnsupportedProgramError, match="missing"):
            _core.compile_unit(["def caller(a: int) -> int:\n    return missing(a)\n"])


class TestIrArtifact:
    def test_the_ir_is_returned_as_readable_json(self) -> None:
        artifact = json.loads(_core.compile_unit([ADD]).ir_artifact)

        # Version 2 since operators started carrying their declared semantics: an artifact
        # written before that spells division as `FloorDiv`, which no longer exists.
        assert artifact["version"] == 2
        assert [f["name"] for f in artifact["functions"]] == ["add"]

    def test_the_artifact_names_no_rust_types(self) -> None:
        # The IR is the stage every backend consumes; a Rust spelling in it would mean the
        # abstraction had already leaked.
        artifact = _core.compile_unit([ADD]).ir_artifact
        for spelling in ("i64", "f64", "String"):
            assert spelling not in artifact

    def test_the_artifact_records_the_fingerprint(self) -> None:
        compiled = _core.compile_unit([ADD])
        assert json.loads(compiled.ir_artifact)["fingerprint"] == compiled.fingerprint


class TestFingerprint:
    def test_formatting_does_not_change_it(self) -> None:
        noisy = (
            "# a leading comment\n"
            "def add(a: int, b: int) -> int:\n"
            "\n"
            "        # an indented comment\n"
            "        return a + b\n"
        )
        assert _core.compile_unit([ADD]).fingerprint == _core.compile_unit([noisy]).fingerprint

    def test_a_changed_body_changes_it(self) -> None:
        changed = "def add(a: int, b: int) -> int:\n    return a - b\n"
        assert _core.compile_unit([ADD]).fingerprint != _core.compile_unit([changed]).fingerprint

    def test_the_module_name_follows_the_fingerprint(self) -> None:
        compiled = _core.compile_unit([ADD])
        assert compiled.fingerprint in compiled.module_name


class TestDiagnostics:
    def test_invalid_python_raises_a_syntax_error(self) -> None:
        with pytest.raises(_core.SourceSyntaxError):
            _core.compile_unit(["def broken(:\n"])

    def test_valid_python_outside_the_subset_raises_a_different_error(self) -> None:
        with pytest.raises(_core.UnsupportedProgramError):
            _core.compile_unit(
                ["def loops(a: int) -> int:\n    while a:\n        pass\n    return a\n"]
            )

    def test_the_two_are_distinguishable(self) -> None:
        # Catching one must not catch the other: a typo and an unsupported feature need different
        # responses from a caller.
        with pytest.raises(_core.SourceSyntaxError):
            _core.compile_unit(["def broken(:\n"])
        assert not issubclass(_core.SourceSyntaxError, _core.UnsupportedProgramError)
        assert not issubclass(_core.UnsupportedProgramError, _core.SourceSyntaxError)

    def test_both_share_one_catchable_base(self) -> None:
        for source in ("def broken(:\n", "def f(a) -> int:\n    return a\n"):
            with pytest.raises(_core.CompilationError):
                _core.compile_unit([source])

    def test_a_compilation_error_carries_line_and_column(self) -> None:
        source = 'def f(a: int) -> int:\n    b = a + 1\n    return "x"\n'
        with pytest.raises(_core.CompilationError) as caught:
            _core.compile_unit([source])

        assert caught.value.line == 3
        assert caught.value.column > 1
        # And the message repeats it, so a bare traceback is still useful.
        assert "3:" in str(caught.value)

    @pytest.mark.parametrize(
        ("source", "expected"),
        [
            ("def f(a, b: int) -> int:\n    return b\n", "a"),
            ("def f(a: int):\n    return a\n", "f"),
            ("def f(a: complex) -> int:\n    return 1\n", "complex"),
            ("def f(a: int) -> int:\n    b = helper(a)\n    return b\n", "b"),
        ],
    )
    def test_the_diagnostic_names_the_offending_thing(self, source: str, expected: str) -> None:
        with pytest.raises(_core.CompilationError, match=expected):
            _core.compile_unit([source])


class TestValidateSource:
    def test_returns_the_function_names(self) -> None:
        assert _core.validate_source(ADD) == ["add"]

    def test_rejects_a_function_outside_the_subset(self) -> None:
        with pytest.raises(_core.CompilationError):
            _core.validate_source("def f(a) -> int:\n    return a\n")

    def test_does_not_resolve_calls(self) -> None:
        # A decorated function may legitimately call one that has not been decorated yet.
        # Resolving here would make acceptance depend on decoration order.
        assert _core.validate_source("def caller(a: int) -> int:\n    return callee(a)\n") == [
            "caller"
        ]


class TestBackendRegistry:
    def test_reserved_backends_are_named_but_not_implemented(self) -> None:
        assert set(_core.backend_names()) >= {"rust", "typescript", "go", "cpp"}
        assert _core.implemented_backends() == ["rust"]

    def test_rust_is_usable(self) -> None:
        _core.check_backend("rust")

    def test_a_reserved_backend_says_it_is_planned(self) -> None:
        with pytest.raises(_core.BackendNotAvailableError, match="not implemented yet"):
            _core.check_backend("typescript")

    def test_an_unknown_backend_lists_what_is_available(self) -> None:
        with pytest.raises(_core.BackendNotAvailableError, match="rust") as caught:
            _core.check_backend("nonesuch")
        # A typo must not be reported as a planned target.
        assert "not implemented yet" not in str(caught.value)

    def test_a_backend_failure_is_not_a_compilation_error(self) -> None:
        # Handling a bad program should not accidentally swallow a bad configuration.
        with pytest.raises(_core.CompylrError):
            _core.compile_unit([ADD], "typescript")
        assert not issubclass(_core.BackendNotAvailableError, _core.CompilationError)

    def test_the_backend_is_checked_before_the_source_is_parsed(self) -> None:
        # The source below is not valid Python; the backend is still the thing to report.
        with pytest.raises(_core.BackendNotAvailableError):
            _core.compile_unit(["def broken(:\n"], "nonesuch")
