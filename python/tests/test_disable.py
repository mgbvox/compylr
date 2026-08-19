"""Turning compilation off for a whole process.

The switch exists for two jobs. The first is answering "is this compylr, or is it my code?" without
editing anything. The second is measurement: a marked function calls other marked functions through
module globals, so an interpreted outer call would still reach compiled inner ones — reaching for
`python_function` gives a number that means nothing, and only a whole process running interpreted
gives one that does.

So the property that matters is that a disabled decorator returns **exactly what it was given**, not
a pass-through wrapper. A wrapper would keep compylr in every traceback and every profile, which is
the opposite of what turning it off is for.
"""

from __future__ import annotations

import compylr
import pytest
from compylr import _config, _manager
from compylr._errors import ConfigurationError


@pytest.fixture(autouse=True)
def fresh() -> None:
    _manager._reset_for_tests()


def sample(n: int) -> int:
    return n * 2


class Sample:
    def __init__(self, v: int) -> None:
        self.v: int = v


class TestReadingTheEnvironment:
    @pytest.mark.parametrize("value", ["1", "true", "TRUE", "yes", "on", " 1 "])
    def test_truthy_values_disable(self, monkeypatch: pytest.MonkeyPatch, value: str) -> None:
        monkeypatch.setenv(_config.DISABLE_ENV, value)
        assert _config.disabled_by_environment()

    @pytest.mark.parametrize("value", ["0", "false", "no", "off", ""])
    def test_falsey_values_do_not(self, monkeypatch: pytest.MonkeyPatch, value: str) -> None:
        monkeypatch.setenv(_config.DISABLE_ENV, value)
        assert not _config.disabled_by_environment()

    def test_unset_is_enabled(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(_config.DISABLE_ENV, raising=False)
        assert not _config.disabled_by_environment()

    def test_an_unrecognised_value_is_an_error(self, monkeypatch: pytest.MonkeyPatch) -> None:
        # Silently meaning "enabled" is exactly the kind of wrongness discovered much later, and
        # being sure which mode you are in is the whole point of the switch.
        monkeypatch.setenv(_config.DISABLE_ENV, "maybe")
        with pytest.raises(ConfigurationError) as caught:
            _config.disabled_by_environment()
        assert "maybe" in str(caught.value)


class TestADisabledDecoratorIsATransparentNoOp:
    def test_a_function_comes_back_unchanged(self) -> None:
        c = compylr.initialize(enabled=False)
        marked = c.compyle(sample)
        assert marked is sample, "the original must come back, not a wrapper around it"
        assert marked(21) == 42

    def test_a_class_comes_back_unchanged(self) -> None:
        c = compylr.initialize(enabled=False)
        marked = c.compyle(Sample)
        assert marked is Sample
        assert marked(3).v == 3

    def test_the_called_form_is_a_no_op_too(self) -> None:
        c = compylr.initialize(enabled=False)
        marked = c.compyle(backend="rust")(sample)
        assert marked is sample

    def test_nothing_is_registered(self) -> None:
        # Nothing to build, so a later ensure_built has nothing to do and no toolchain is needed.
        c = compylr.initialize(enabled=False)
        c.compyle(sample)
        assert c._sources == {}
        assert c._functions == {}

    def test_a_program_outside_the_subset_is_not_even_validated(self) -> None:
        # Turning compylr off has to work when the reason for turning it off is that compylr
        # rejects the code -- otherwise the switch is useless in the case it exists for.
        c = compylr.initialize(enabled=False)

        @c.compyle
        def unsupported(n: int) -> int:
            return n**2  # exponentiation is outside the subset

        assert unsupported(4) == 16

    def test_the_environment_switches_it_without_an_argument(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # A project can be run interpreted from the outside, without editing it.
        monkeypatch.setenv(_config.DISABLE_ENV, "1")
        c = compylr.initialize()
        assert not c.enabled
        assert c.compyle(sample) is sample

    def test_an_explicit_argument_beats_the_environment(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setenv(_config.DISABLE_ENV, "1")
        assert compylr.initialize(enabled=True).enabled


class TestSwitchingMidProject:
    def test_re_initializing_with_the_opposite_mode_is_refused(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        # The members marked before the change would otherwise be in a different mode from the
        # ones marked after, in one process, which nothing downstream could make sense of.
        compylr.initialize(enabled=True)
        with pytest.raises(ConfigurationError) as caught:
            compylr.initialize(enabled=False)
        assert "enabled" in str(caught.value)

    def test_re_initializing_with_the_same_mode_returns_the_same_manager(self) -> None:
        first = compylr.initialize(enabled=False)
        assert compylr.initialize(enabled=False) is first


class TestPrecompilingSaysWhenItIsDisabled:
    def test_it_does_not_look_like_an_empty_project(self, tmp_path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
        # "nothing marked" would send the user looking for a decorator that is right where they
        # left it. The report has to name the switch instead.
        monkeypatch.setenv(_config.DISABLE_ENV, "1")
        (tmp_path / "m.py").write_text(
            "import compylr\n"
            f"c = compylr.initialize(root={str(tmp_path / '.compylr')!r})\n"
            "\n"
            "@c.compyle\n"
            "def triple(n: int) -> int:\n"
            "    return n * 3\n"
        )
        from compylr import _precompile

        report = _precompile.precompile(tmp_path)
        assert report.disabled
        assert not report.built
        assert _config.DISABLE_ENV in _precompile._describe(report)
        assert _precompile.main(["compyle", str(tmp_path)]) == 3
