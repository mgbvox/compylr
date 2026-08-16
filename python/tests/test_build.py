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
from compylr._build import BuildPipeline
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

    def test_it_defaults_to_the_working_directory(self) -> None:
        assert BuildPipeline().paths.root == Path.cwd() / ".compylr"


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
        pipeline.paths.state.write_text(
            json.dumps(
                {
                    "version": 1,
                    "fingerprint": "abc123",
                    "module_name": "compylr_generated_abc123",
                    "functions": ["f"],
                }
            )
        )
        assert pipeline.cached_fingerprint() == "abc123"
        assert pipeline.cached_module_name() == "compylr_generated_abc123"
