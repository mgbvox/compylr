"""The user-facing API: initialization, both decorator forms, and settings resolution.

Nothing here builds anything. Marking a function validates it and registers it; that is a
separable step from compiling, and testing it separately is what keeps this file fast.
"""

from __future__ import annotations

from pathlib import Path

import compylr
import pytest
from compylr import _core
from compylr._config import Settings


class TestInitialize:
    def test_returns_a_manager_with_the_given_settings(self) -> None:
        c = compylr.initialize(backend="rust", llm_assist=False)
        assert isinstance(c, compylr.Manager)
        assert c.settings == Settings(backend="rust", llm_assist=False)

    def test_defaults_need_no_arguments(self) -> None:
        c = compylr.initialize()
        assert c.settings.backend == compylr.DEFAULT_BACKEND
        assert c.settings.llm_assist is False

    def test_repeating_the_same_settings_returns_the_same_manager(self) -> None:
        # One manager per project is what keeps every marked function in one shared artifact.
        first = compylr.initialize(backend="rust")
        second = compylr.initialize(backend="rust")
        assert first is second

    def test_reconfiguring_with_an_unusable_backend_reports_the_backend(self) -> None:
        # Validation runs before the conflict check, which is the right order: naming a backend
        # that cannot be used at all is the more immediate mistake.
        compylr.initialize(backend="rust")
        with pytest.raises(_core.BackendNotAvailableError):
            compylr.initialize(backend="typescript")

    def test_conflicting_reconfiguration_is_refused(self) -> None:
        # Only one settings combination currently validates -- rust, assist off -- so a genuine
        # conflict cannot be produced through the public API yet. The guard still has to work the
        # moment a second backend lands, so the stored settings are forged to reach it.
        manager = compylr.initialize(backend="rust")
        object.__setattr__(manager.settings, "backend", "go")

        with pytest.raises(compylr.ConfigurationError, match="already initialized"):
            compylr.initialize(backend="rust")

    def test_an_unknown_backend_is_refused_immediately(self) -> None:
        with pytest.raises(_core.BackendNotAvailableError, match="rust"):
            compylr.initialize(backend="nonesuch")

    def test_a_reserved_backend_says_it_is_planned(self) -> None:
        with pytest.raises(_core.BackendNotAvailableError, match="not implemented yet"):
            compylr.initialize(backend="typescript")


class TestLlmAssist:
    def test_enabling_it_globally_is_refused(self) -> None:
        with pytest.raises(compylr.ConfigurationError, match="not implemented yet"):
            compylr.initialize(llm_assist=True)

    def test_enabling_it_for_one_function_is_refused(self) -> None:
        c = compylr.initialize()
        with pytest.raises(compylr.ConfigurationError, match="not implemented yet"):

            @c.compyle(llm_assist=True)
            def f(a: int) -> int:
                return a

    def test_disabling_it_is_silent(self) -> None:
        c = compylr.initialize(llm_assist=False)

        @c.compyle(llm_assist=False)
        def f(a: int) -> int:
            return a

        assert f.settings.llm_assist is False

    def test_omitting_it_is_silent(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a

        assert f.settings.llm_assist is False


class TestDecoratorForms:
    def test_bare_form(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a

        assert isinstance(f, compylr.CompiledFunction)
        assert f.settings == c.settings

    def test_called_form_with_no_arguments_is_equivalent(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def bare(a: int) -> int:
            return a

        @c.compyle()
        def called(a: int) -> int:
            return a

        assert bare.settings == called.settings

    def test_called_form_with_settings(self) -> None:
        c = compylr.initialize()

        @c.compyle(backend="rust")
        def f(a: int) -> int:
            return a

        assert f.settings.backend == "rust"


class TestSettingsResolution:
    def test_an_override_applies_to_one_function_only(self) -> None:
        c = compylr.initialize(backend="rust")

        @c.compyle(backend="rust")
        def overridden(a: int) -> int:
            return a

        @c.compyle
        def inherited(a: int) -> int:
            return a

        assert overridden.settings.backend == "rust"
        assert inherited.settings.backend == c.settings.backend
        # The manager's own defaults are untouched by an override.
        assert c.settings.backend == "rust"

    def test_unspecified_settings_are_inherited(self) -> None:
        c = compylr.initialize(backend="rust")

        @c.compyle(llm_assist=False)
        def f(a: int) -> int:
            return a

        assert f.settings.backend == "rust", "naming one setting must not reset the others"

    def test_a_reserved_backend_fails_at_the_decorator_that_named_it(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.BackendNotAvailableError, match="not implemented yet"):

            @c.compyle(backend="typescript")
            def f(a: int) -> int:
                return a

    def test_an_unknown_backend_fails_at_the_decorator_that_named_it(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.BackendNotAvailableError):

            @c.compyle(backend="nonesuch")
            def f(a: int) -> int:
                return a


class TestRejection:
    def test_a_missing_annotation_is_rejected_when_marked(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError, match="a"):

            @c.compyle
            def f(a, b: int) -> int:  # type: ignore[no-untyped-def]
                return b

    def test_an_unsupported_construct_is_rejected_when_marked(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError):

            @c.compyle
            def f(a: int) -> int:
                while a:
                    pass
                return a

    def test_rejection_carries_a_location(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError) as caught:

            @c.compyle
            def f(a: int) -> int:
                b = a + 1  # noqa: F841 -- the point is that line 3 is where it fails
                return "x"  # type: ignore[return-value]

        assert caught.value.line >= 1
        assert caught.value.column >= 1

    def test_rejection_happens_before_any_call(self) -> None:
        # The failure must point at the decorator, not at a call site reached much later.
        # A `for` loop rather than an unused import: an autofixer would delete the import and
        # quietly turn this into a test of nothing, which is how the compiler fixtures were once
        # broken.
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError):

            @c.compyle
            def f(a: int) -> int:
                for _ in range(a):
                    pass
                return a

    def test_a_call_to_another_marked_function_needs_no_annotation(self) -> None:
        # The arrangement the decorator always produces: each function is its own source, so this
        # is a call across sources. Refusing it here would mean the inference compylr advertises
        # worked everywhere except through its main interface.
        c = compylr.initialize()

        @c.compyle
        def double(n: int) -> int:
            return n * 2

        @c.compyle
        def uses(n: int) -> int:
            doubled = double(n)
            return doubled + 1

        assert "uses" in c._sources

    def test_decoration_order_does_not_matter(self) -> None:
        # The caller is marked first, so its callee is not merely in another source -- it is not
        # yet registered at all.
        c = compylr.initialize()

        @c.compyle
        def uses(n: int) -> int:
            doubled = double(n)
            return doubled + 1

        @c.compyle
        def double(n: int) -> int:
            return n * 2

        assert set(c._sources) == {"uses", "double"}

    def test_only_the_undetermined_category_is_deferred(self) -> None:
        # Deferring must not become tolerating. Every other violation still fails at the
        # decorator, which is where the user can see what caused it.
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError) as caught:

            @c.compyle
            def loops(n: int) -> int:
                for _ in range(n):
                    pass
                return n

        assert caught.value.code != "undetermined_binding"

    def test_the_diagnostic_code_is_readable(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError) as caught:

            @c.compyle
            def f(a, b: int) -> int:  # type: ignore[no-untyped-def]
                return b

        assert caught.value.code == "missing_annotation"

    def test_there_is_no_silent_fallback(self) -> None:
        # A rejected function must not quietly remain interpreted: the user asked for compilation
        # and would otherwise be measuring the wrong thing.
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError):

            @c.compyle
            def f(a) -> int:  # type: ignore[no-untyped-def]
                return a

        assert "f" not in [name for name in c._sources]


class TestMarkedFunctionsAreOrdinaryObjects:
    def test_identity_attributes_are_preserved(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def named(a: int) -> int:
            return a

        assert named.__name__ == "named"
        assert named.__module__ == __name__
        # Compared against the original rather than against literal types: this module uses
        # `from __future__ import annotations`, so they are strings here. What matters is that
        # marking a function did not lose them.
        assert named.__annotations__ == named.python_function.__annotations__
        assert set(named.__annotations__) == {"a", "return"}

    def test_a_docstring_does_not_prevent_compilation(self) -> None:
        # Was a strict xfail until docstrings were accepted. Most code worth compiling is
        # documented, so this was the single largest thing standing between the decorator and
        # real use.
        c = compylr.initialize()

        @c.compyle
        def documented(a: int) -> int:
            """Return the argument."""
            return a

        assert documented.__doc__ == "Return the argument."

    def test_a_multi_line_docstring_is_accepted(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def described(a: int) -> int:
            """Scale a value.

            A longer explanation, of the kind house style asks for.
            """
            return a * 2

        assert described.__doc__ is not None
        assert "Scale a value." in described.__doc__

    def test_a_stray_string_statement_is_still_rejected(self) -> None:
        # The exception is narrow on purpose: only the first statement, and only a string. A
        # discarded value anywhere else is dead code or an inexpressible side effect.
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError):

            @c.compyle
            def stray(a: int) -> int:
                """A real docstring."""
                "but this one is just a discarded value"  # noqa: B018
                return a

    def test_the_original_function_is_reachable(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a * 3

        assert f.__wrapped__(2) == 6
        assert f.python_function(2) == 6

    def test_it_is_usable_as_a_callable(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a

        assert callable(f)

    def test_repr_says_whether_it_has_been_built(self) -> None:
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a

        assert "not built yet" in repr(f)
        assert "f" in repr(f)


class TestRegistration:
    def test_marking_two_different_functions_of_the_same_name_is_refused(self) -> None:
        # They share one compiled module, so the names have to be unique.
        c = compylr.initialize()

        @c.compyle
        def f(a: int) -> int:
            return a

        with pytest.raises(compylr.ConfigurationError, match="unique"):

            @c.compyle
            def f(a: int) -> int:  # noqa: F811
                return a * 2

    def test_artifacts_are_rooted_where_the_manager_was_told(self, build_root: Path) -> None:
        c = compylr.initialize(root=build_root)
        assert c.paths.root == build_root
        assert c.paths.ir.parent.parent == build_root
