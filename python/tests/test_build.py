"""The build pipeline's decisions, without actually building.

Everything here is about what the pipeline does *around* the toolchain: where files go, when a
rebuild is warranted, and what a user is told when the machine cannot build at all. Those are the
parts that must behave the same on a machine with no Rust installed, so none of these tests compile
anything.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from compylr import _core
from compylr._build import BuildPipeline, discover_root
from compylr._errors import BuildError, ToolchainMissingError


@pytest.fixture
def pipeline(build_root: Path) -> BuildPipeline:
    return BuildPipeline(build_root)


class TestPaths:
    def test_everything_lives_under_one_root(self, pipeline: BuildPipeline) -> None:
        root = pipeline.paths.root
        for path in (
            pipeline.paths.ir,
            pipeline.paths.crate,
            pipeline.paths.target_source,
            pipeline.paths.manifest,
            pipeline.paths.dist,
            pipeline.paths.lib,
            pipeline.paths.state,
        ):
            assert root in path.parents or path == root

    def test_an_explicit_root_skips_discovery(self, tmp_path: Path) -> None:
        explicit = tmp_path / "somewhere" / ".compylr"
        assert BuildPipeline(explicit).paths.root == explicit


class TestRootDiscovery:
    """Finding the artifact directory from anywhere inside a project.

    Rooting it at the working directory means running the same project from a subdirectory builds
    a second copy from scratch -- which reads as a cache bug and is really just a path.
    """

    def test_an_existing_artifact_directory_is_found_from_a_subdirectory(
        self, tmp_path: Path
    ) -> None:
        (tmp_path / ".compylr").mkdir()
        nested = tmp_path / "src" / "deep"
        nested.mkdir(parents=True)

        assert discover_root(nested) == tmp_path / ".compylr"

    def test_a_pyproject_marks_the_root(self, tmp_path: Path) -> None:
        (tmp_path / "pyproject.toml").write_text("[project]\nname = 'x'\n")
        nested = tmp_path / "pkg"
        nested.mkdir()

        assert discover_root(nested) == tmp_path / ".compylr"

    def test_an_existing_artifact_directory_wins_over_a_higher_pyproject(
        self, tmp_path: Path
    ) -> None:
        # A project already built somewhere should keep using what it built.
        (tmp_path / "pyproject.toml").write_text("[project]\nname = 'x'\n")
        inner = tmp_path / "inner"
        inner.mkdir()
        (inner / ".compylr").mkdir()

        assert discover_root(inner) == inner / ".compylr"

    def test_no_marker_falls_back_to_the_starting_directory(self, tmp_path: Path) -> None:
        # Reaching the filesystem root without a marker must not select an arbitrary ancestor.
        bare = tmp_path / "bare"
        bare.mkdir()
        assert discover_root(bare) == bare / ".compylr"

    def test_the_search_stops_at_the_filesystem_root(self, tmp_path: Path) -> None:
        bare = tmp_path / "a" / "b" / "c"
        bare.mkdir(parents=True)
        found = discover_root(bare)
        # Whatever it picked, it is inside the temporary tree rather than somewhere on the machine.
        assert tmp_path in found.parents or found == bare / ".compylr"

    def test_discovery_is_used_when_no_root_is_given(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        (tmp_path / "pyproject.toml").write_text("[project]\nname = 'x'\n")
        nested = tmp_path / "pkg"
        nested.mkdir()
        monkeypatch.chdir(nested)

        assert BuildPipeline().paths.root == tmp_path / ".compylr"


class TestToolchainChecks:
    def test_a_missing_rust_toolchain_is_named_with_a_fix(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr("compylr._build.shutil.which", lambda tool: None)
        with pytest.raises(ToolchainMissingError) as caught:
            BuildPipeline.check_toolchain()

        assert "cargo" in str(caught.value)
        assert "rustup.rs" in str(caught.value), "the error must say how to fix it"

    def test_a_missing_build_tool_is_named_with_a_fix(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(
            "compylr._build.shutil.which",
            lambda tool: "/usr/bin/cargo" if tool == "cargo" else None,
        )
        with pytest.raises(ToolchainMissingError) as caught:
            BuildPipeline.check_toolchain()

        assert "maturin" in str(caught.value)
        assert "install" in str(caught.value)

    def test_a_missing_toolchain_is_a_build_error(self, monkeypatch: pytest.MonkeyPatch) -> None:
        # Callers handling "compylr could not build this" should not have to enumerate causes.
        monkeypatch.setattr("compylr._build.shutil.which", lambda tool: None)
        with pytest.raises(BuildError):
            BuildPipeline.check_toolchain()

    def test_the_check_happens_before_any_work(
        self, pipeline: BuildPipeline, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr("compylr._build.shutil.which", lambda tool: None)

        def fail(*args: object, **kwargs: object) -> None:
            raise AssertionError("nothing should be written before the toolchain is checked")

        monkeypatch.setattr(BuildPipeline, "write_artifacts", fail)
        with pytest.raises(ToolchainMissingError):
            pipeline.build(object())  # type: ignore[arg-type]


def _state(**overrides: object) -> dict[str, object]:
    """Recorded build state that this compylr would accept."""
    from compylr._build import _STATE_VERSION, _compiler_version

    return {
        "version": _STATE_VERSION,
        "compylr": _compiler_version(),
        "fingerprint": "abc123",
        "module_name": "compylr_generated_abc123",
        "functions": ["f"],
    } | overrides


class TestCache:
    def test_no_state_means_no_cached_build(self, pipeline: BuildPipeline) -> None:
        assert pipeline.cached_fingerprint() is None
        assert pipeline.cached_module_name() is None

    def test_unreadable_state_is_treated_as_absent(self, pipeline: BuildPipeline) -> None:
        # A truncated or hand-edited file must not be able to make the pipeline skip a build.
        pipeline.paths.root.mkdir(parents=True, exist_ok=True)
        pipeline.paths.state.write_text("{not json")
        assert pipeline.cached_fingerprint() is None

    def test_state_from_another_layout_version_is_ignored(self, pipeline: BuildPipeline) -> None:
        pipeline.paths.root.mkdir(parents=True, exist_ok=True)
        pipeline.paths.state.write_text(
            json.dumps({"version": 999, "fingerprint": "abc", "module_name": "m"})
        )
        assert pipeline.cached_fingerprint() is None
        assert pipeline.cached_module_name() is None

    def test_a_recorded_build_is_reported(self, pipeline: BuildPipeline) -> None:
        pipeline.paths.root.mkdir(parents=True, exist_ok=True)
        pipeline.paths.state.write_text(json.dumps(_state()))
        assert pipeline.cached_fingerprint() == "abc123"
        assert pipeline.cached_module_name() == "compylr_generated_abc123"

    def test_state_from_another_compylr_is_not_reused(self, pipeline: BuildPipeline) -> None:
        # The rebuild key is a fingerprint of the IR, which is what makes reformatting free. A new
        # compylr emitting different code from the same IR would otherwise reuse the old artifact
        # forever, and the user would see last version's generated code with no way to know.
        pipeline.paths.root.mkdir(parents=True, exist_ok=True)
        pipeline.paths.state.write_text(json.dumps(_state(compylr="0.0.0-not-this-one")))
        assert pipeline.cached_fingerprint() is None
        assert pipeline.cached_module_name() is None


class TestPassConfiguration:
    """The same program built by a different set of passes is a different artifact.

    The fingerprint identifies the *program* and deliberately does not move when a pass is turned
    on -- otherwise enabling one would look exactly like the user editing their code. That leaves
    the pass set as the thing build state has to record, or an artifact built by an older compylr
    with a different default would be reused forever.
    """

    def test_state_records_the_passes_that_ran(self, tmp_path: Path) -> None:
        pipeline = BuildPipeline(tmp_path)
        compiled = _core.compile_unit(["def double(n: int) -> int:\n    return n * 2\n"])
        pipeline._record_success(compiled)

        state = json.loads((tmp_path / "state.json").read_text())
        assert state["passes"] == list(compiled.passes)
        assert state["passes"], "the default configuration runs at least one pass"

    def test_a_build_by_a_different_pass_set_is_not_reused(self, tmp_path: Path) -> None:
        pipeline = BuildPipeline(tmp_path)
        compiled = _core.compile_unit(["def double(n: int) -> int:\n    return n * 2\n"])
        pipeline._record_success(compiled)

        assert pipeline.cached_module_name(list(compiled.passes)) == compiled.module_name
        assert pipeline.cached_module_name(["some-other-pass"]) is None

    def test_asking_without_a_pass_set_still_reads_the_name(self, tmp_path: Path) -> None:
        # The narrower question is for reuse decisions; the broader one is for reporting.
        pipeline = BuildPipeline(tmp_path)
        compiled = _core.compile_unit(["def double(n: int) -> int:\n    return n * 2\n"])
        pipeline._record_success(compiled)
        assert pipeline.cached_module_name() == compiled.module_name
