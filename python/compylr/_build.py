"""Turning generated source into an importable extension module.

The shape on disk, all under one directory so it can be inspected or deleted as a unit:

    .compylr/
      ir/unit.json      the IR, for reading
      crate/            the shared generated crate
      dist/             built wheels
      lib/              staged module, when installing into the environment is not appropriate
      state.json        the fingerprint of the last successful build

One artifact serves the whole project. Compiling per function would multiply build cost by the
number of decorated functions and would stop them calling each other.
"""

from __future__ import annotations

import importlib
import importlib.util
import json
import shutil
import subprocess
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType

from ._core import CompiledUnit
from ._errors import BuildError, ToolchainMissingError

__all__ = ["BuildPaths", "BuildPipeline"]

#: Version of the on-disk build state, so a future layout change is detected rather than misread.
#:
#: Bumped to 2 when the generated crate went from one `lib.rs` to a file per concern. Without the
#: bump an unchanged project would skip the rebuild and keep the old single file on disk -- never
#: compiled against, but shown to anyone who opens it, contradicting the documented layout.
_STATE_VERSION = 2


@dataclass(frozen=True, slots=True)
class BuildPaths:
    """Where a project's build artifacts live."""

    root: Path

    @property
    def ir(self) -> Path:
        return self.root / "ir" / "unit.json"

    @property
    def crate(self) -> Path:
        return self.root / "crate"

    @property
    def src(self) -> Path:
        return self.crate / "src"

    @property
    def target_source(self) -> Path:
        """The file holding the translated functions.

        Named specifically because it is the one worth reading: `lib.rs` is module declarations,
        `compat.rs` is identical in every project, and `bindings.rs` is boundary plumbing.
        """
        return self.src / "generated.rs"

    @property
    def manifest(self) -> Path:
        return self.crate / "Cargo.toml"

    @property
    def dist(self) -> Path:
        return self.root / "dist"

    @property
    def lib(self) -> Path:
        return self.root / "lib"

    @property
    def state(self) -> Path:
        return self.root / "state.json"


#: Files that mark the top of a project, in the order they are preferred.
#:
#: An existing artifact directory wins: a project that has been built once should keep using what
#: it built, even when a `pyproject.toml` sits further up. `.git` is deliberately absent — a
#: monorepo holding several projects would collapse them into one artifact directory, which is
#: worse than the problem being solved.
_PROJECT_MARKERS = (".compylr", "pyproject.toml")


def discover_root(start: Path | None = None) -> Path:
    """Locate a project's artifact directory by searching upward for a marker.

    The directory is a property of the project, not of the shell. Rooting it at the working
    directory means running the same project from a subdirectory builds a second copy from
    scratch, which reads as a cache bug and is really just a path.

    The search stops at the filesystem root and falls back to the working directory, so a script
    in an unmarked directory still works rather than selecting an arbitrary ancestor.
    """
    here = (start or Path.cwd()).resolve()
    for directory in (here, *here.parents):
        for marker in _PROJECT_MARKERS:
            candidate = directory / marker
            if candidate.exists():
                # The marker may be the artifact directory itself, or the file that names the
                # project it should sit beside.
                return candidate if marker == ".compylr" else directory / ".compylr"
    return here / ".compylr"


class BuildPipeline:
    """Builds a compiled unit and hands back the imported module."""

    def __init__(self, root: Path | None = None) -> None:
        # An explicit location skips discovery entirely: a caller who says where artifacts go has
        # already answered the question the search exists to answer.
        self.paths = BuildPaths(Path(root) if root is not None else discover_root())

    # -- toolchain -----------------------------------------------------------------------

    @staticmethod
    def check_toolchain() -> None:
        """Fail with an actionable message when a required tool is absent.

        Checked before any work so a missing toolchain is reported as such, rather than as a
        file-not-found error from deep inside a build.
        """
        if shutil.which("cargo") is None:
            raise ToolchainMissingError(
                "The Rust toolchain (cargo)",
                "Install it from https://rustup.rs, then try again.",
            )
        if shutil.which("maturin") is None:
            raise ToolchainMissingError(
                "maturin",
                "Install it with `uv pip install maturin` or `pipx install maturin`.",
            )

    # -- cache ---------------------------------------------------------------------------

    def cached_module_name(self) -> str | None:
        """The module built by the last successful build, if any."""
        try:
            state = json.loads(self.paths.state.read_text())
        except (OSError, json.JSONDecodeError):
            return None
        if state.get("version") != _STATE_VERSION:
            return None
        name = state.get("module_name")
        return name if isinstance(name, str) else None

    def cached_fingerprint(self) -> str | None:
        """The fingerprint of the last successful build, if any."""
        try:
            state = json.loads(self.paths.state.read_text())
        except (OSError, json.JSONDecodeError):
            return None
        if state.get("version") != _STATE_VERSION:
            return None
        value = state.get("fingerprint")
        return value if isinstance(value, str) else None

    def _record_success(self, compiled: CompiledUnit) -> None:
        # Written only after the module has been built AND imported, so a failed build never
        # leaves state claiming success and causing the next run to skip the work.
        self.paths.state.write_text(
            json.dumps(
                {
                    "version": _STATE_VERSION,
                    "fingerprint": compiled.fingerprint,
                    "module_name": compiled.module_name,
                    "functions": compiled.function_names,
                },
                indent=2,
            )
            + "\n"
        )

    # -- build ---------------------------------------------------------------------------

    def write_artifacts(self, compiled: CompiledUnit) -> None:
        """Write the IR and the generated crate.

        Both intermediates land on disk on every build. A transpiler whose intermediate stages are
        invisible cannot be debugged, and "read the generated Rust" is the first thing anyone will
        want to do when a result surprises them.
        """
        self.paths.ir.parent.mkdir(parents=True, exist_ok=True)
        self.paths.ir.write_text(compiled.ir_artifact + "\n")

        # `src/` holds nothing hand-authored, so it is rewritten wholesale rather than diffed
        # against a record of the last build. A file a previous build wrote and this one did not
        # would still compile, and could still be reachable if a module declaration outlived it --
        # a failure that presents as "my change had no effect".
        #
        # Scoped to `src/`: the manifest, the cargo configuration, and `target/` sit outside it,
        # and losing `target/` would make every build a cold build.
        if self.paths.src.exists():
            shutil.rmtree(self.paths.src)
        for relative, contents in compiled.target_sources.items():
            path = self.paths.crate / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)

        self.paths.crate.mkdir(parents=True, exist_ok=True)
        self.paths.manifest.write_text(compiled.manifest)

        # An extension module resolves the interpreter's symbols at load time instead of linking
        # libpython. On macOS the linker has to be told those symbols may be missing.
        cargo_config = self.paths.crate / ".cargo"
        cargo_config.mkdir(parents=True, exist_ok=True)
        (cargo_config / "config.toml").write_text(
            "[target.aarch64-apple-darwin]\n"
            'rustflags = ["-C", "link-arg=-undefined", "-C", "link-arg=dynamic_lookup"]\n'
            "[target.x86_64-apple-darwin]\n"
            'rustflags = ["-C", "link-arg=-undefined", "-C", "link-arg=dynamic_lookup"]\n'
        )

    def build(self, compiled: CompiledUnit) -> ModuleType:
        """Build, make importable, import, and record success."""
        self.check_toolchain()
        self.write_artifacts(compiled)

        wheel = self._build_wheel()
        self._make_importable(wheel, compiled.module_name)

        importlib.invalidate_caches()
        try:
            module = importlib.import_module(compiled.module_name)
        except ImportError as error:  # pragma: no cover - only on a broken build
            raise BuildError(
                f"built {compiled.module_name} but could not import it", str(error)
            ) from error

        self._record_success(compiled)
        return module

    def _build_wheel(self) -> Path:
        self.paths.dist.mkdir(parents=True, exist_ok=True)
        for stale in self.paths.dist.glob("*.whl"):
            stale.unlink()

        result = subprocess.run(
            [
                "maturin",
                "build",
                "--release",
                "--manifest-path",
                str(self.paths.manifest),
                "--out",
                str(self.paths.dist),
            ],
            # Run from inside the crate so cargo picks up its `.cargo/config.toml`. Cargo reads
            # that file relative to the working directory, not to the manifest, so building from
            # the project root would silently drop the macOS link arguments and fail at link time
            # with undefined Python symbols.
            cwd=self.paths.crate,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            # The toolchain's own diagnostics are the actionable part, so they are carried through
            # rather than summarised away.
            raise BuildError("the generated crate failed to build", result.stderr or result.stdout)

        wheels = sorted(self.paths.dist.glob("*.whl"))
        if not wheels:
            raise BuildError("the build reported success but produced no wheel", result.stdout)
        return wheels[-1]

    def _make_importable(self, wheel: Path, module_name: str) -> None:
        """Install the wheel, or stage it where it can be imported from.

        Installing is right inside a virtual environment, which is what the project's own
        environment is. Outside one, installing would mutate a system interpreter that compylr was
        never given permission to touch, so the module is unpacked under `.compylr/lib` and that
        directory is put on the path instead. Both leave the module importable, which is the
        requirement; only one of them is a decision the user did not ask for.
        """
        if _in_virtual_environment():
            installer = (
                ["uv", "pip", "install"]
                if shutil.which("uv")
                else [sys.executable, "-m", "pip", "install"]
            )
            result = subprocess.run(
                [*installer, "--force-reinstall", str(wheel)],
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode == 0:
                return
            # Fall through to staging rather than failing: an unwritable environment is a
            # deployment detail, not a reason the compiled code cannot be used.

        self.paths.lib.mkdir(parents=True, exist_ok=True)
        for stale in self.paths.lib.iterdir():
            if stale.name.startswith("compylr_generated_"):
                if stale.is_dir():
                    shutil.rmtree(stale)
                else:
                    stale.unlink()
        with zipfile.ZipFile(wheel) as archive:
            archive.extractall(self.paths.lib)

        staged = str(self.paths.lib.resolve())
        if staged not in sys.path:
            sys.path.insert(0, staged)
        del module_name  # named for readability at the call site


def _in_virtual_environment() -> bool:
    """Whether the running interpreter is a virtual environment."""
    return sys.prefix != sys.base_prefix
