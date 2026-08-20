"""Compiling a project ahead of its first call.

Discovery imports the project. That is inherent — a decorator registers when it runs — and the tests
here pin both halves of the bargain: everything marked is found, and nothing outside the root or
inside an environment is imported. A precompiler that silently misses a function is worse than none,
because the symptom is a slow first call rather than an error.
"""

from __future__ import annotations

import textwrap
from pathlib import Path

import compylr
import pytest
from compylr import _manager, _precompile
from compylr._errors import ConfigurationError
from conftest import needs_toolchain

MARKED = '''
import compylr

c = compylr.initialize(root={root!r})


@c.compyle
def {name}(n: int) -> int:
    return n * {factor}
'''


def write(root: Path, relative: str, body: str) -> Path:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(body))
    return path


@pytest.fixture(autouse=True)
def fresh() -> None:
    _manager._reset_for_tests()


class TestPackages:
    """A package must import the way it imports at runtime.

    Nothing here was covered before, and that absence is exactly what let the bug ship: every
    fixture was a flat directory of standalone modules, so no relative import was ever executed.
    `compylr compyle` could not import a package's `__init__.py` at all, and the demo has been
    reporting two failures that nothing looked at.
    """

    def test_a_package_whose_init_imports_a_sibling(self, tmp_path: Path) -> None:
        write(tmp_path, "pkg/__init__.py", "from . import work\n\n__all__ = ['work']\n")
        write(tmp_path, "pkg/work.py", MARKED.format(root=str(tmp_path), name="doubled", factor=2))

        report = _precompile.precompile(tmp_path)

        assert report.failures == [], "a relative import inside a package must resolve"
        assert "doubled" in report.functions

    def test_a_nested_package_resolves_every_ancestor(self, tmp_path: Path) -> None:
        write(tmp_path, "outer/__init__.py", "from . import inner\n")
        write(tmp_path, "outer/inner/__init__.py", "from . import leaf\n")
        write(
            tmp_path,
            "outer/inner/leaf.py",
            MARKED.format(root=str(tmp_path), name="tripled", factor=3),
        )

        report = _precompile.precompile(tmp_path)

        assert report.failures == []
        assert "tripled" in report.functions

    def test_enumeration_order_does_not_decide_success(self, tmp_path: Path) -> None:
        # `Aaa` sorts before `__init__.py`, because `A` is 0x41 and `_` is 0x5F. A fix that only
        # sorted `__init__.py` first would pass every other test here and fail this one.
        write(tmp_path, "pkg/__init__.py", "VALUE = 1\n")
        write(tmp_path, "pkg/Aaa/__init__.py", "from .. import VALUE\n")
        write(
            tmp_path,
            "pkg/Aaa/deep.py",
            MARKED.format(root=str(tmp_path), name="quadrupled", factor=4),
        )

        report = _precompile.precompile(tmp_path)

        assert report.failures == []
        assert "quadrupled" in report.functions

    def test_a_package_that_genuinely_raises_is_still_reported(self, tmp_path: Path) -> None:
        # The fix must not swallow real failures by making every import succeed vacuously.
        write(tmp_path, "pkg/__init__.py", "from . import missing_entirely\n")
        write(tmp_path, "pkg/fine.py", MARKED.format(root=str(tmp_path), name="fine", factor=5))

        report = _precompile.precompile(tmp_path)

        assert len(report.failures) == 1
        assert "pkg/__init__.py" in report.failures[0].module
        assert "fine" in report.functions, "one broken module must not stop the rest"


class TestDiscovery:
    def test_every_marked_member_across_modules_is_found(self, tmp_path: Path) -> None:
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "a.py", MARKED.format(root=artifacts, name="one", factor=2))
        write(tmp_path, "pkg/b.py", MARKED.format(root=artifacts, name="two", factor=3))
        write(
            tmp_path,
            "pkg/c.py",
            f'''
            import compylr

            c = compylr.initialize(root={artifacts!r})


            @c.compyle
            class Box:
                def __init__(self, v: int) -> None:
                    self.v: int = v

                def get(self) -> int:
                    return self.v
            ''',
        )
        report = _precompile.precompile(tmp_path)
        assert set(report.functions) == {"one", "two"}
        assert report.classes == ["Box"], "a marked class is found alongside functions"

    def test_only_modules_beneath_the_root_are_imported(self, tmp_path: Path) -> None:
        artifacts = str(tmp_path / "project" / ".compylr")
        project = tmp_path / "project"
        write(project, "inside.py", MARKED.format(root=artifacts, name="inside", factor=2))
        write(tmp_path, "outside.py", MARKED.format(root=artifacts, name="outside", factor=9))
        report = _precompile.precompile(project)
        assert report.functions == ["inside"]

    def test_environments_and_caches_are_skipped(self, tmp_path: Path) -> None:
        # Precompiling a small project must not import an arbitrary dependency tree.
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "real.py", MARKED.format(root=artifacts, name="real", factor=2))
        for skipped in (".venv", "__pycache__", ".git", "build", "node_modules"):
            write(tmp_path, f"{skipped}/hidden.py", "raise AssertionError('must not be imported')")
        report = _precompile.precompile(tmp_path)
        assert report.functions == ["real"]
        assert report.failures == [], "a skipped directory is not an import failure"

    def test_a_module_that_raises_is_reported_and_the_rest_proceed(self, tmp_path: Path) -> None:
        # One broken module must not stop the project being precompiled, and naming it keeps the
        # omission visible rather than silent.
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "good.py", MARKED.format(root=artifacts, name="good", factor=2))
        write(tmp_path, "bad.py", "raise RuntimeError('boom')")
        report = _precompile.precompile(tmp_path)
        assert report.functions == ["good"]
        assert len(report.failures) == 1
        assert report.failures[0].module == "bad.py"
        assert "boom" in report.failures[0].reason


class TestTheReport:
    def test_nothing_marked_is_not_an_error_and_says_so(self, tmp_path: Path) -> None:
        write(tmp_path, "plain.py", "VALUE = 1\n")
        report = _precompile.precompile(tmp_path)
        assert report.found_nothing
        assert not report.built
        assert "nothing marked" in _precompile._describe(report)

    def test_a_missing_root_is_reported(self, tmp_path: Path) -> None:
        with pytest.raises(ConfigurationError):
            _precompile.precompile(tmp_path / "absent")

    def test_the_summary_carries_the_import_failure_count(self, tmp_path: Path) -> None:
        write(tmp_path, "bad.py", "raise RuntimeError('boom')")
        report = _precompile.precompile(tmp_path)
        assert "1 module(s) failed to import" in _precompile._describe(report)


@needs_toolchain
@pytest.mark.slow
class TestBuildingAheadOfACall:
    def test_a_project_builds_with_nothing_called(self, tmp_path: Path) -> None:
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "m.py", MARKED.format(root=artifacts, name="triple", factor=3))
        report = _precompile.precompile(tmp_path)
        assert report.built, "the build must happen without any call"
        assert (tmp_path / ".compylr" / "state.json").is_file()

    def test_an_already_current_project_is_not_rebuilt(self, tmp_path: Path) -> None:
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "m.py", MARKED.format(root=artifacts, name="triple", factor=3))
        assert _precompile.precompile(tmp_path).built

        _manager._reset_for_tests()
        again = _precompile.precompile(tmp_path)
        assert not again.built, "an unchanged project must reuse, not rebuild"
        assert again.marked == 1

    def test_reformatting_does_not_rebuild_but_an_edit_does(self, tmp_path: Path) -> None:
        artifacts = str(tmp_path / ".compylr")
        module = write(tmp_path, "m.py", MARKED.format(root=artifacts, name="triple", factor=3))
        assert _precompile.precompile(tmp_path).built

        # A comment changes the text and not the IR, which is the whole point of fingerprinting
        # structure rather than source.
        _manager._reset_for_tests()
        module.write_text(module.read_text().replace("return n * 3", "return n * 3  # a comment"))
        assert not _precompile.precompile(tmp_path).built

        _manager._reset_for_tests()
        module.write_text(module.read_text().replace("n * 3", "n * 4"))
        assert _precompile.precompile(tmp_path).built, "an edit must be picked up"

    def test_precompiling_then_calling_performs_no_second_build(self, tmp_path: Path) -> None:
        # The property the whole change exists for: after precompiling, the first call is fast
        # because there is nothing left to do.
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "m.py", MARKED.format(root=artifacts, name="triple", factor=3))
        precompiled = _precompile.precompile(tmp_path)
        assert precompiled.built

        manager = _manager._active_manager()
        assert manager is not None
        fingerprint = manager._built_fingerprint
        assert manager._functions["triple"](7) == 21
        assert manager._built_fingerprint == fingerprint, "calling must not have rebuilt"

    def test_building_ahead_lands_in_the_projects_own_directory(self, tmp_path: Path) -> None:
        artifacts = tmp_path / "elsewhere"
        write(tmp_path, "m.py", MARKED.format(root=str(artifacts), name="triple", factor=3))
        assert _precompile.precompile(tmp_path).built
        assert (artifacts / "state.json").is_file()


class TestTheCommand:
    def test_help_states_that_discovery_imports(self, capsys: pytest.CaptureFixture[str]) -> None:
        # Importing runs module-level code. A user must not discover that by surprise.
        with pytest.raises(SystemExit):
            _precompile.main(["--help"])
        assert "imports every module" in capsys.readouterr().out

    def test_a_missing_root_exits_unsuccessfully(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        assert _precompile.main(["compyle", str(tmp_path / "absent")]) == 2
        assert "compylr:" in capsys.readouterr().err

    def test_nothing_found_is_distinguishable_from_success(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        # Not success: a container image that precompiles nothing has failed at what it was for,
        # and the symptom would otherwise appear much later as a slow first request.
        write(tmp_path, "plain.py", "VALUE = 1\n")
        assert _precompile.main(["compyle", str(tmp_path)]) == 3

    @needs_toolchain
    @pytest.mark.slow
    def test_a_successful_run_reports_what_it_did(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        artifacts = str(tmp_path / ".compylr")
        write(tmp_path, "m.py", MARKED.format(root=artifacts, name="triple", factor=3))
        assert _precompile.main(["compyle", str(tmp_path)]) == 0
        out = capsys.readouterr().out
        assert "found 1 function(s)" in out
        assert "built" in out

        _manager._reset_for_tests()
        assert _precompile.main(["compyle", str(tmp_path)]) == 0
        assert "reused" in capsys.readouterr().out, "reuse must be distinguishable from building"


def test_the_module_is_exported() -> None:
    assert compylr.precompile is _precompile.precompile
