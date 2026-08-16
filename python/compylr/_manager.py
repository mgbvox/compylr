"""The manager and the decorator: what a user actually touches.

Two timings matter and they are deliberately different.

**Marking a function validates it immediately.** Lowering one function is microseconds, so there is
no reason to accept a function compylr cannot compile and fail at first call instead — by then the
traceback points at a call site rather than at the decorator that caused it.

**Building happens on first call.** Building at decoration would compile the project once per
decorated function. First call is the earliest moment at which every module-level decorator has
run *and* a result is actually needed.

One consequence is worth knowing: a call to a function that was never decorated fails at build
time, not at decoration. That is not an oversight. Callee resolution happens over the assembled
unit precisely so that results do not depend on which function was decorated first.
"""

from __future__ import annotations

import functools
from collections.abc import Callable
from pathlib import Path
from typing import Any, Generic, ParamSpec, TypeVar

from . import _core
from ._build import BuildPipeline
from ._config import INHERIT, Settings
from ._errors import ConfigurationError
from ._source import capture_source

__all__ = ["CompiledFunction", "Manager", "initialize"]

#: The one diagnostic category the decorator defers to build time.
#:
#: Matched on the stable code rather than the message, which is prose and free to be reworded.
_DEFERRED_UNTIL_BUILD = "undetermined_binding"

P = ParamSpec("P")
R = TypeVar("R")


class CompiledFunction(Generic[P, R]):
    """A marked function.

    Keeps the identifying attributes callers and tooling read, and exposes the original through
    `__wrapped__` so compiled and interpreted behaviour can be compared — which is what makes
    "the compiled result matches the interpreted one" testable at all.
    """

    def __init__(self, function: Callable[P, R], manager: Manager, settings: Settings) -> None:
        self._function = function
        self._manager = manager
        self._settings = settings
        self._compiled: Callable[..., R] | None = None
        functools.update_wrapper(self, function)
        # Since PEP 649 (3.14) a function's annotations are computed lazily from `__annotate__`,
        # and `update_wrapper` copies that rather than `__annotations__`. The descriptor that
        # would evaluate it belongs to function objects, so on a plain instance the annotations
        # would simply be missing. Copying the resolved mapping keeps introspection working.
        self.__annotations__ = getattr(function, "__annotations__", {})

    @property
    def settings(self) -> Settings:
        """The settings this function compiles under."""
        return self._settings

    @property
    def python_function(self) -> Callable[P, R]:
        """The original, uncompiled implementation."""
        return self._function

    def __call__(self, *args: P.args, **kwargs: P.kwargs) -> R:
        if self._compiled is None:
            self._compiled = self._manager._resolve(self._function.__name__)
        return self._compiled(*args, **kwargs)

    def __repr__(self) -> str:
        state = "compiled" if self._compiled is not None else "not built yet"
        return f"<compylr function {self._function.__name__!r} ({state})>"


class Manager:
    """A project's compylr configuration and the functions marked under it."""

    def __init__(self, settings: Settings, root: Path | None = None) -> None:
        self._settings = settings
        self._pipeline = BuildPipeline(root)
        self._sources: dict[str, str] = {}
        self._functions: dict[str, CompiledFunction[Any, Any]] = {}
        self._module: Any = None
        self._built_fingerprint: str | None = None

    @property
    def settings(self) -> Settings:
        """The project-wide defaults."""
        return self._settings

    @property
    def paths(self) -> Any:
        """Where this project's build artifacts live."""
        return self._pipeline.paths

    def compyle(
        self,
        function: Callable[P, R] | None = None,
        *,
        backend: str | object = INHERIT,
        llm_assist: bool | object = INHERIT,
    ) -> Any:
        """Mark a function for compilation.

        Usable bare (`@c.compyle`) or called (`@c.compyle(backend=...)`). Settings not named are
        inherited from the manager, so naming one does not silently reset the others.
        """
        settings = self._settings.override(backend=backend, llm_assist=llm_assist)

        def mark(target: Callable[P, R]) -> CompiledFunction[P, R]:
            return self._register(target, settings)

        # Bare form: the decorated function arrives as the first positional argument.
        if function is not None:
            return mark(function)
        return mark

    def _register(self, function: Callable[P, R], settings: Settings) -> CompiledFunction[P, R]:
        source = capture_source(function)
        # Raises here, with a line and column, if the function is outside the subset -- which is
        # the point of validating at all: the failure should point at the decorator, not at a call
        # site reached much later.
        #
        # One category is deferred. A binding whose initializer calls a function this source does
        # not define cannot be typed yet, because each decorated function is captured as its own
        # source and its callees live in other ones. Refusing here would demand an annotation for
        # `doubled = double(n)` in exactly the arrangement the decorator always produces. The
        # build sees every source at once and types it then, so nothing goes unchecked -- it is
        # checked once there is enough information to check it with.
        #
        # The cost, stated plainly: if that callee is never marked, the failure arrives at the
        # first call rather than here. That is the same lateness unresolved callees already have,
        # since they are likewise only resolvable across the assembled unit.
        try:
            _core.validate_source(source)
        except _core.UnsupportedProgramError as error:
            if getattr(error, "code", None) != _DEFERRED_UNTIL_BUILD:
                raise

        name = function.__name__
        if name in self._sources and self._sources[name] != source:
            raise ConfigurationError(
                f"two different functions named {name!r} were marked; names must be unique "
                f"within a project because they share one compiled module"
            )

        self._sources[name] = source
        wrapper = CompiledFunction(function, self, settings)
        self._functions[name] = wrapper
        # A newly marked function changes the unit, so whatever was built no longer covers it.
        self._built_fingerprint = None
        return wrapper

    def _resolve(self, name: str) -> Callable[..., Any]:
        """Return the compiled implementation of `name`, building if needed."""
        module = self.ensure_built()
        try:
            return getattr(module, name)  # type: ignore[no-any-return]
        except AttributeError as error:  # pragma: no cover - implies a codegen defect
            raise ConfigurationError(f"{name!r} is missing from the compiled module") from error

    def ensure_built(self) -> Any:
        """Build the project if what is loaded does not cover every marked function."""
        backends = {f.settings.backend for f in self._functions.values()}
        if len(backends) > 1:
            raise ConfigurationError(
                "functions in one project are marked for different backends "
                f"({', '.join(sorted(backends))}); a project compiles to one shared artifact"
            )
        backend = backends.pop() if backends else self._settings.backend

        compiled = _core.compile_unit(list(self._sources.values()), backend)

        # Already loaded and current: nothing to do. This is the path every run after the first
        # takes, and the reason reformatting does not cost a rebuild.
        if self._module is not None and self._built_fingerprint == compiled.fingerprint:
            return self._module

        if (
            self._pipeline.cached_fingerprint() == compiled.fingerprint
            and self._pipeline.cached_module_name() == compiled.module_name
        ):
            module = self._import_cached(compiled.module_name)
            if module is not None:
                self._module = module
                self._built_fingerprint = compiled.fingerprint
                return module

        self._module = self._pipeline.build(compiled)
        self._built_fingerprint = compiled.fingerprint
        return self._module

    def _import_cached(self, module_name: str) -> Any:
        """Import a previously built module without rebuilding, if it is still there."""
        import importlib
        import sys

        staged = str(self._pipeline.paths.lib.resolve())
        if self._pipeline.paths.lib.is_dir() and staged not in sys.path:
            sys.path.insert(0, staged)
        try:
            importlib.invalidate_caches()
            return importlib.import_module(module_name)
        except ImportError:
            return None


#: The process-wide manager, so a project compiles to one shared artifact.
_MANAGER: Manager | None = None


def initialize(
    backend: str = "rust",
    llm_assist: bool = False,
    *,
    root: Path | None = None,
) -> Manager:
    """Configure compylr for this project and return its manager.

    Calling it again with the same settings returns the same manager, which is what keeps every
    decorated function in one shared artifact. Calling it with *different* settings is refused
    rather than silently re-pointing a project that is already partly configured — the functions
    marked before the change would otherwise compile under settings their author never chose.
    """
    global _MANAGER
    settings = Settings(backend=backend, llm_assist=llm_assist)

    if _MANAGER is None:
        _MANAGER = Manager(settings, root)
        return _MANAGER

    if _MANAGER.settings != settings:
        raise ConfigurationError(
            f"compylr is already initialized with {_MANAGER.settings}, and re-initializing with "
            f"{settings} would change the settings of a project that is already configured"
        )
    return _MANAGER


def _reset_for_tests() -> None:
    """Drop the process-wide manager.

    Only for tests: process-wide state and independent test cases are otherwise incompatible.
    """
    global _MANAGER
    _MANAGER = None
