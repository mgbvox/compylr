"""Compiling a project ahead of the first call.

A marked function compiles on its first call, which makes that call slow. For anything that starts
under a request — a container image, a serverless handler, a CLI a user is waiting on — that cost
lands in the wrong place. Precompiling moves it to build time.

Discovery **imports** the project's modules. That is inherent rather than a shortcut: a decorator
only registers when it runs, so nothing that reads source text can know which functions are marked
without reimplementing what `@c.compyle` means — a notion that drifts from the runtime's the moment
someone aliases the import or decorates conditionally. A precompiler that silently misses a function
is worse than none, because the symptom is a slow first call rather than an error.

The cost is bounded instead of hidden: only modules beneath the given root, never installed
packages, skipping the directories that hold environments, caches, and build output.
"""

from __future__ import annotations

import importlib.machinery
import importlib.util
import sys
from dataclasses import dataclass, field
from pathlib import Path

from ._config import DISABLE_ENV
from ._errors import ConfigurationError

#: Directories never descended into.
#:
#: An environment or a cache holds other people's code, and importing it would mean precompiling a
#: small project pulled in an arbitrary dependency tree.
SKIPPED_DIRECTORIES = frozenset(
    {".venv", "venv", ".git", ".hg", "__pycache__", ".compylr", ".mypy_cache", ".pytest_cache",
     ".ruff_cache", "build", "dist", "target", "node_modules", ".tox", ".eggs"}
)


@dataclass(frozen=True)
class ImportFailure:
    """A module that could not be imported."""

    module: str
    reason: str


@dataclass
class Report:
    """What a precompile run found and did.

    Facts rather than formatted text: the command owns presentation, so the two forms cannot
    disagree about what happened.
    """

    root: Path
    modules_imported: int = 0
    functions: list[str] = field(default_factory=list)
    classes: list[str] = field(default_factory=list)
    failures: list[ImportFailure] = field(default_factory=list)
    built: bool = False
    #: Whether the project turned compylr off, via `initialize(enabled=False)` or the environment.
    disabled: bool = False

    @property
    def marked(self) -> int:
        """How many members were found."""
        return len(self.functions) + len(self.classes)

    @property
    def found_nothing(self) -> bool:
        """Whether the run had nothing to compile.

        Deliberately not success. A script that precompiles in a container image and silently
        compiles nothing has failed at the thing it was there for, and the symptom would otherwise
        surface much later as a slow first request.
        """
        return self.marked == 0


def _module_files(root: Path) -> list[Path]:
    """Every importable module beneath `root`, in the order Python would import them.

    A package's own `__init__.py` comes before anything inside the package, and before any
    subpackage. That is not cosmetic: importing `pkg.sub` at runtime runs `pkg/__init__.py` first,
    so a subpackage reading `from .. import VALUE` sees a package whose module has executed. Walking
    in plain sorted order gets this right by accident for lowercase names and wrong for a directory
    like `Aaa`, which sorts before `__init__.py` because `A` is 0x41 and `_` is 0x5F.
    """
    found: list[Path] = []

    def walk(directory: Path) -> None:
        entries = sorted(directory.iterdir())

        initializer = directory / "__init__.py"
        if initializer.is_file():
            found.append(initializer)

        for entry in entries:
            if entry.is_file() and entry.suffix == ".py" and entry.name not in _NOT_MODULES:
                found.append(entry)

        # Subdirectories last, so this package exists before anything below it is imported.
        for entry in entries:
            skipped = entry.name in SKIPPED_DIRECTORIES or entry.name.startswith(".")
            if entry.is_dir() and not skipped:
                walk(entry)

    walk(root)
    return found


#: Files that are not modules to import in their own right.
#:
#: `__init__.py` is excluded here because it is appended first, ahead of its siblings, rather than
#: in name order; `setup.py` because running it is a packaging action, not a discovery one.
_NOT_MODULES = frozenset({"__init__.py", "setup.py"})


def _module_name(path: Path, root: Path) -> str:
    """A unique, importable name for a file, derived from its path below the root."""
    relative = path.relative_to(root).with_suffix("")
    parts = [part for part in relative.parts if part != "__init__"]
    return ".".join([PRIVATE_ROOT, *parts]) if parts else PRIVATE_ROOT


#: The private prefix every discovered module is imported under.
#:
#: Private so that importing a project's `utils.py` cannot shadow an installed `utils`, and so a
#: project cannot be affected by whatever else the precompiling process has imported.
PRIVATE_ROOT = "_compylr_precompile"


def _ensure_ancestors(name: str) -> None:
    """Make every package above `name` exist, so a relative import inside it can resolve.

    A module imported as `_compylr_precompile.pkg.mod` resolves `from . import sibling` by looking
    up `_compylr_precompile.pkg` in `sys.modules`. If nothing put it there, the import fails with a
    `ModuleNotFoundError` naming a package the user never wrote — which is what happened to every
    package's `__init__.py` until this existed.

    Ancestors are created on demand rather than by importing `__init__.py` files first. Relying on
    order would work today and break the first time a directory's name sorts before `__init__.py`,
    which `Aaa` does: `A` is 0x41 and `_` is 0x5F. A placeholder created here is replaced by the
    real module when its own file is imported; a submodule already bound stays reachable, because
    relative imports resolve through `sys.modules` rather than through the parent's attributes.
    """
    parts = name.split(".")
    for depth in range(1, len(parts)):
        ancestor = ".".join(parts[:depth])
        if ancestor in sys.modules:
            continue
        package = importlib.util.module_from_spec(
            importlib.machinery.ModuleSpec(ancestor, loader=None, is_package=True)
        )
        sys.modules[ancestor] = package


def _import_file(path: Path, name: str) -> None:
    """Import one file under a private name, so it cannot collide with the caller's modules."""
    _ensure_ancestors(name)

    # An `__init__.py` has to be loaded as a *package*, not as a plain module. Without a search
    # location its `__package__` resolves to its parent, so `from . import sibling` inside
    # `_compylr_precompile.pkg` would look for `_compylr_precompile.sibling` — a name nothing ever
    # binds. With one, `__package__` is the package's own name and the lookup is right.
    locations = [str(path.parent)] if path.name == "__init__.py" else None
    spec = importlib.util.spec_from_file_location(name, path, submodule_search_locations=locations)
    if spec is None or spec.loader is None:  # pragma: no cover - only for unreadable files
        raise ImportError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        # Left out of sys.modules so a later, fixed import is not served the broken one.
        sys.modules.pop(name, None)
        raise


def precompile(root: Path | str) -> Report:
    """Import every module beneath `root` and build whatever they mark.

    Returns a report rather than printing one. A module that raises on import is recorded and
    skipped: one broken module should not stop the rest of a project being precompiled, and naming
    it keeps the omission visible instead of silent.
    """
    root = Path(root).resolve()
    if not root.is_dir():
        raise ConfigurationError(f"{root} is not a directory")

    report = Report(root=root)

    # The root goes on the path so the project's own intra-project imports resolve the way they do
    # when it runs normally.
    inserted = str(root)
    if inserted not in sys.path:
        sys.path.insert(0, inserted)

    for path in _module_files(root):
        name = _module_name(path, root)
        try:
            _import_file(path, name)
        except BaseException as error:  # noqa: BLE001 - a module may raise anything on import
            report.failures.append(ImportFailure(module=str(path.relative_to(root)),
                                                 reason=f"{type(error).__name__}: {error}"))
            continue
        report.modules_imported += 1

    from ._manager import CompiledClass, _active_manager

    manager = _active_manager()
    if manager is None:
        return report

    # Precompiling with compylr disabled would find nothing and report it as an empty project,
    # sending the user to look for a missing decorator that is right where they left it.
    if not manager.enabled:
        report.disabled = True
        return report

    for name, wrapper in sorted(manager._functions.items()):
        if isinstance(wrapper, CompiledClass):
            report.classes.append(name)
        else:
            report.functions.append(name)

    if report.found_nothing:
        return report

    # Raises whatever a call-triggered build would, carrying the toolchain's output, so the two
    # paths fail identically.
    manager.ensure_built()
    # Asked rather than inferred: a fresh process reusing a cached artifact moves the fingerprint
    # from unset to set without having built anything, so comparing fingerprints would call every
    # first run in a process a build.
    report.built = manager.last_build_invoked_toolchain
    return report


def _describe(report: Report) -> str:
    """The human-readable form of a report."""
    lines = [f"compylr: {report.root}"]
    lines.append(
        f"  imported {report.modules_imported} module(s); "
        f"found {len(report.functions)} function(s) and {len(report.classes)} class(es)"
    )
    if report.failures:
        # In the summary, not only the detail: a count buried below is a count nobody reads.
        lines.append(f"  {len(report.failures)} module(s) failed to import:")
        for failure in report.failures:
            lines.append(f"    {failure.module}: {failure.reason}")
    if report.disabled:
        lines.append(
            f"  compylr is disabled for this project (see {DISABLE_ENV}); nothing was compiled"
        )
    elif report.found_nothing:
        lines.append("  nothing marked for compilation")
    elif report.built:
        lines.append("  built")
    else:
        lines.append("  already current; reused the existing build")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    """The `compylr` command.

    A thin wrapper: it parses, calls [`precompile`], formats, and chooses an exit status. Anything
    decided only here would be a place the command and the function could disagree.
    """
    import argparse

    parser = argparse.ArgumentParser(
        prog="compylr",
        description=(
            "Compile a project ahead of its first run, so no call pays the build cost."
        ),
        epilog=(
            "Discovery imports every module beneath the root, because a decorator only registers "
            "when it runs. Module-level code therefore executes. Environments, caches, and build "
            "output are skipped, and installed packages are never followed."
        ),
    )
    parser.add_argument("command", choices=["compyle"], help="what to do")
    parser.add_argument("root", nargs="?", default=".", help="the project root to compile")
    args = parser.parse_args(argv)

    try:
        report = precompile(args.root)
    except ConfigurationError as error:
        print(f"compylr: {error}", file=sys.stderr)
        return 2
    except Exception as error:  # noqa: BLE001 - a build failure carries the toolchain's output
        print(f"compylr: {error}", file=sys.stderr)
        return 1

    quiet = report.found_nothing or report.disabled
    stream = sys.stderr if quiet else sys.stdout
    print(_describe(report), file=stream)
    # Distinguishable outcomes: built or reused, nothing compiled, and failure.
    return 3 if quiet else 0
