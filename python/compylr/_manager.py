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
from typing import Any, Generic, ParamSpec, TypeVar, overload

from . import _core
from ._build import BuildPipeline
from ._config import INHERIT, Settings, disabled_by_environment
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


class CompiledClass:
    """A marked class.

    Instantiating it builds the project if needed and returns an instance of the **compiled** type,
    not of the Python original. That is the whole point: the object Python holds is the translated
    struct, so a method mutating an attribute persists between calls — where a collection passed to
    a compiled function is a copy and cannot.

    The original stays reachable through `python_class`, so compiled and interpreted behaviour can
    be compared the same way they can for a function.
    """

    def __init__(self, cls: type, manager: Manager, settings: Settings) -> None:
        self._class = cls
        self._manager = manager
        self._settings = settings
        # Typed as a plain callable rather than `type`: what the module exposes is a PyO3 class
        # object, and calling it is all this needs of it.
        self._compiled: Callable[..., Any] | None = None
        self.__name__ = cls.__name__
        self.__qualname__ = cls.__qualname__
        self.__doc__ = cls.__doc__
        self.__module__ = cls.__module__

    @property
    def settings(self) -> Settings:
        """The settings this class compiles under."""
        return self._settings

    @property
    def python_class(self) -> type:
        """The original, uncompiled class."""
        return self._class

    def __call__(self, *args: Any, **kwargs: Any) -> Any:
        if self._compiled is None:
            self._compiled = self._manager._resolve(self._class.__name__)
        return self._compiled(*args, **kwargs)

    def __repr__(self) -> str:
        state = "compiled" if self._compiled is not None else "not built yet"
        return f"<compylr class {self._class.__name__!r} ({state})>"


class Manager:
    """A project's compylr configuration and the functions marked under it."""

    def __init__(
        self, settings: Settings, root: Path | None = None, *, enabled: bool = True
    ) -> None:
        self._settings = settings
        self._enabled = enabled
        self._pipeline = BuildPipeline(root)
        self._sources: dict[str, str] = {}
        self._functions: dict[str, CompiledFunction[Any, Any]] = {}
        self._module: Any = None
        self._built_fingerprint: str | None = None
        #: Whether the last `ensure_built` actually ran the toolchain, as opposed to reusing what
        #: was already loaded or already on disk.
        self.last_build_invoked_toolchain = False

    @property
    def settings(self) -> Settings:
        """The project-wide defaults."""
        return self._settings

    @property
    def enabled(self) -> bool:
        """Whether marking a function compiles it.

        When false, `@c.compyle` hands back exactly what it was given: nothing is validated, nothing
        is registered, and no build is attempted. The project runs as ordinary Python.
        """
        return self._enabled

    @property
    def paths(self) -> Any:
        """Where this project's build artifacts live."""
        return self._pipeline.paths

    # `type` is itself callable, so mypy reads the two as overlapping. The callable form comes
    # first and the class form is spelled with a distinct return, which is enough to keep a
    # decorated function's signature intact -- the case that actually matters for callers.
    @overload
    def compyle(self, function: Callable[P, R], /) -> CompiledFunction[P, R]: ...

    @overload
    def compyle(self, function: type, /) -> CompiledClass: ...

    @overload
    def compyle(
        self,
        function: None = None,
        *,
        backend: str | object = ...,
        llm_assist: bool | object = ...,
    ) -> Callable[[Any], Any]: ...

    def compyle(
        self,
        function: Any = None,
        *,
        backend: str | object = INHERIT,
        llm_assist: bool | object = INHERIT,
    ) -> Any:
        """Mark a function or a class for compilation.

        Usable bare (`@c.compyle`) or called (`@c.compyle(backend=...)`). Settings not named are
        inherited from the manager, so naming one does not silently reset the others.

        When the manager is disabled — `initialize(enabled=False)`, or `COMPYLR_DISABLE=1` in the
        environment — this returns the target untouched and the project runs interpreted.
        """
        # Disabled: hand back exactly what was given, before touching settings at all. Returning
        # the original rather than a pass-through wrapper matters — a wrapper would keep compylr in
        # every traceback and every profile, which is the opposite of what turning it off is for.
        if not self._enabled:
            return (lambda target: target) if function is None else function

        settings = self._settings.override(backend=backend, llm_assist=llm_assist)

        def mark(target: Callable[P, R]) -> Any:
            return self._register(target, settings)

        # Bare form: the decorated function arrives as the first positional argument.
        if function is not None:
            return mark(function)
        return mark

    def _register(self, function: Any, settings: Settings) -> Any:
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
        # A class and a function are marked the same way and share one build; only what the
        # wrapper does on call differs.
        wrapper: Any = (
            CompiledClass(function, self, settings)
            if isinstance(function, type)
            else CompiledFunction(function, self, settings)
        )
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
        """Build the project if what is loaded does not cover every marked function.

        Records whether the toolchain actually ran, in `last_build_invoked_toolchain`. A caller
        cannot infer that from the fingerprint: a fresh process reusing a cached artifact moves the
        fingerprint from unset to set without having built anything.
        """
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
            self.last_build_invoked_toolchain = False
            return self._module

        if (
            self._pipeline.cached_fingerprint() == compiled.fingerprint
            and self._pipeline.cached_module_name(list(compiled.passes))
            == compiled.module_name
        ):
            module = self._import_cached(compiled.module_name)
            if module is not None:
                self._module = module
                self._built_fingerprint = compiled.fingerprint
                self.last_build_invoked_toolchain = False
                return module

        self._module = self._pipeline.build(compiled)
        self._built_fingerprint = compiled.fingerprint
        self.last_build_invoked_toolchain = True
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


def _active_manager() -> Manager | None:
    """The process-wide manager, if one has been created.

    Precompiling needs it after importing a project, which is the only way to learn what that
    project marked -- a decorator registers when it runs and not before.
    """
    return _MANAGER


def initialize(
    backend: str = "rust",
    llm_assist: bool = False,
    *,
    root: Path | None = None,
    enabled: bool | None = None,
) -> Manager:
    """Configure compylr for this project and return its manager.

    Calling it again with the same settings returns the same manager, which is what keeps every
    decorated function in one shared artifact. Calling it with *different* settings is refused
    rather than silently re-pointing a project that is already partly configured — the functions
    marked before the change would otherwise compile under settings their author never chose.

    `enabled=False` turns compilation off: every `@c.compyle` hands back what it was given and the
    project runs as ordinary Python. Left unset it follows `COMPYLR_DISABLE` in the environment, so
    a project can be switched to interpreted from the outside without editing it.
    """
    global _MANAGER
    settings = Settings(backend=backend, llm_assist=llm_assist)
    resolved = not disabled_by_environment() if enabled is None else enabled

    if _MANAGER is None:
        _MANAGER = Manager(settings, root, enabled=resolved)
        return _MANAGER

    if _MANAGER.enabled != resolved:
        state = "enabled" if _MANAGER.enabled else "disabled"
        raise ConfigurationError(
            f"compylr is already initialized and {state}; re-initializing with the opposite would "
            f"leave the members marked so far in a different mode than the ones marked after"
        )

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
