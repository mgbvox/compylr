"""Collections across the boundary.

The conversions themselves are PyO3's; what is worth testing here is that the types compylr
declares map onto the Python types a caller expects, that the two new failure modes arrive as the
exceptions Python raises, and the two divergences this change accepted on purpose.
"""

from __future__ import annotations

import pytest
from compylr import _core

TOTAL = "def total(xs: list[int]) -> int:\n    return xs[0] + len(xs)\n"


def compile_unit(source: str) -> _core.CompiledUnit:
    """Compile one source under the inherited behavior."""
    return _core.compile_unit([(source, {})])


class TestAnnotations:
    @pytest.mark.parametrize(
        "annotation",
        ["list[int]", "dict[str, int]", "set[int]", "tuple[int, str]", "dict[str, list[int]]"],
    )
    def test_collection_annotations_are_accepted(self, annotation: str) -> None:
        assert _core.validate_source(f"def f(a: {annotation}) -> int:\n    return 1\n") == ["f"]

    @pytest.mark.parametrize(
        ("annotation", "reason"),
        [
            ("list", "an element type that is not written down is not a type"),
            ("dict[str]", "wrong parameter count"),
            ("list[complex]", "unsupported parameter"),
            ("dict[float, int]", "a float key can never be retrieved once it is nan"),
            ("set[float]", "same, for set elements"),
            ("frozenset[int]", "a generic compylr does not model"),
        ],
    )
    def test_unsupported_annotations_are_rejected(self, annotation: str, reason: str) -> None:
        with pytest.raises(_core.CompilationError):
            _core.validate_source(f"def f(a: {annotation}) -> int:\n    return 1\n")


class TestLiterals:
    def test_literals_infer_their_types(self) -> None:
        assert _core.validate_source(
            'def f() -> int:\n    xs = [1, 2]\n    d = {"a": 1}\n    s = {1}\n    return 1\n'
        ) == ["f"]

    def test_mismatched_elements_are_rejected(self) -> None:
        with pytest.raises(_core.CompilationError, match="same type"):
            _core.validate_source('def f() -> int:\n    xs = [1, "a"]\n    return 1\n')

    def test_an_empty_literal_needs_an_annotation(self) -> None:
        with pytest.raises(_core.CompilationError):
            _core.validate_source("def f() -> int:\n    xs = []\n    return 1\n")

    def test_an_annotated_empty_literal_is_accepted(self) -> None:
        assert _core.validate_source("def f() -> int:\n    xs: list[int] = []\n    return 1\n") == [
            "f"
        ]


class TestSubscriptAndLen:
    def test_a_computed_tuple_index_is_rejected(self) -> None:
        # Each position has its own type, so a computed index has no single answer.
        with pytest.raises(_core.CompilationError, match="literal"):
            _core.validate_source("def f(t: tuple[int, str], i: int) -> int:\n    return t[i]\n")

    def test_slicing_is_rejected(self) -> None:
        with pytest.raises(_core.CompilationError, match="[Ss]lic"):
            _core.validate_source("def f(xs: list[int]) -> int:\n    ys = xs[1:2]\n    return 1\n")

    def test_len_of_a_number_is_rejected(self) -> None:
        with pytest.raises(_core.CompilationError):
            _core.validate_source("def f(n: int) -> int:\n    return len(n)\n")

    def test_a_function_named_len_is_reserved(self) -> None:
        with pytest.raises(_core.CompilationError, match="reserved"):
            _core.validate_source("def len(x: int) -> int:\n    return x\n")


class TestGeneratedSpellings:
    def test_collections_spell_recursively(self) -> None:
        # `FastMap` rather than `HashMap`: generated containers carry the hasher the backend
        # selects. It is an alias for `HashMap` with that hasher, so it is the same container —
        # but the spelling is what says the choice was made rather than inherited.
        compiled = compile_unit("def f(d: dict[str, list[int]]) -> int:\n    return 1\n")
        source = compiled.target_sources["src/generated.rs"]
        assert "FastMap<String, Vec<i64>>" in source

    def test_the_ir_artifact_names_no_rust_types(self) -> None:
        # The IR is what every backend consumes; a Rust spelling there is a leak.
        artifact = compile_unit(TOTAL).ir_artifact
        for spelling in ("Vec<", "HashMap", "HashSet", "FastMap", "FastSet", "i64"):
            assert spelling not in artifact
