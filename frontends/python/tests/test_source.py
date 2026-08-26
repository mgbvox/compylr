"""Recovering compilable source from a live function object.

Two things have to be undone before `inspect.getsource` output stands on its own: the decorator
that triggered the capture, and indentation from an enclosing scope. Both are silent failures if
missed — the source simply fails to parse, and the diagnostic points at code the user did not
write.
"""

from __future__ import annotations

import pytest
from compylr import _core
from compylr._source import capture_source


def module_level(a: int, b: int) -> int:
    return a + b


class TestDecoratorStripping:
    def test_a_decorator_line_is_removed(self) -> None:
        def marker(f: object) -> object:
            return f

        @marker
        def decorated(a: int) -> int:
            return a

        source = capture_source(decorated)  # type: ignore[arg-type]
        assert source.startswith("def decorated")
        assert "@marker" not in source

    def test_several_decorators_are_removed(self) -> None:
        def marker(f: object) -> object:
            return f

        @marker
        @marker
        def decorated(a: int) -> int:
            return a

        source = capture_source(decorated)  # type: ignore[arg-type]
        assert source.startswith("def decorated")
        assert "@marker" not in source

    def test_a_multi_line_decorator_is_removed(self) -> None:
        # Scanning for the first line starting with `def` would work here, but not below.
        def marker(**kwargs: object) -> object:
            return lambda f: f

        @marker(
            first=1,
            second=2,
        )
        def decorated(a: int) -> int:
            return a

        source = capture_source(decorated)  # type: ignore[arg-type]
        assert source.startswith("def decorated")
        assert "first=1" not in source

    def test_a_decorator_containing_the_word_def_is_still_removed(self) -> None:
        # The case that defeats a textual scan: the decorator's own argument contains `def`.
        def marker(**kwargs: object) -> object:
            return lambda f: f

        @marker(note="def not a definition")
        def decorated(a: int) -> int:
            return a

        source = capture_source(decorated)  # type: ignore[arg-type]
        assert source.startswith("def decorated")
        assert "not a definition" not in source


class TestDedenting:
    def test_a_nested_function_is_dedented(self) -> None:
        def outer() -> object:
            def inner(a: int) -> int:
                return a * 2

            return inner

        source = capture_source(outer())  # type: ignore[arg-type]
        assert source.startswith("def inner")
        # And the result must actually be compilable, which is the point of dedenting.
        assert _core.validate_source(source) == ["inner"]

    def test_a_module_level_function_is_unchanged(self) -> None:
        source = capture_source(module_level)
        assert source.startswith("def module_level")
        assert _core.validate_source(source) == ["module_level"]


class TestCapturedSourceIsCompilable:
    def test_the_captured_source_lowers(self) -> None:
        def marker(f: object) -> object:
            return f

        @marker
        def target(a: int, b: int) -> int:
            c = a * b
            return c + 1

        assert _core.validate_source(capture_source(target)) == ["target"]  # type: ignore[arg-type]

    def test_source_that_cannot_be_retrieved_raises(self) -> None:
        # A function built by exec has no retrievable text, and there is nothing to compile.
        namespace: dict[str, object] = {}
        exec("def generated(a: int) -> int:\n    return a\n", namespace)  # noqa: S102
        with pytest.raises(OSError):
            capture_source(namespace["generated"])  # type: ignore[arg-type]
