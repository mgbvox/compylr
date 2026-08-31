"""compylr — the Python host for a polyglot transpiler and compiler.

compylr reads a strict, fully annotated subset of a source language into a language-neutral IR,
emits a target language from it, and generates the bridge that makes the result callable from
where it came. This package is Python's end of that: it is the frontend that reads your code and
the host that calls the result back.

    import compylr

    c = compylr.initialize(backend="rust", llm_assist=False)

    @c.compyle
    def add(a: int, b: int) -> int:
        return a + b

The first call to a marked function compiles every marked function in the project into one shared
Rust extension and swaps the compiled implementation in. Later runs reuse it, keyed on a
fingerprint of the IR rather than of the source text, so comments and reformatting do not trigger
a rebuild.

`backend` names the target language. `rust` is the one that completes the round trip from Python
today, because calling a compiled unit back requires a host bridge for the `(source, target)`
pair and `(python, rust)` is the pair that has one. Asking for a target compylr can emit but not
yet call back — Go is the live example — fails saying exactly that, which is a different answer
from an unknown or a merely reserved target.

Compiling requires a Rust toolchain and maturin on the machine running the project; `uv add
compylr` on its own installs the compiler, not the ability to build what it generates.
"""

from __future__ import annotations

from ._config import DEFAULT_BACKEND, DISABLE_ENV, Behavior, Settings, disabled_by_environment
from ._core import (
    BackendNotAvailableError,
    CompilationError,
    CompylrError,
    SourceSyntaxError,
    UnsupportedProgramError,
    backend_names,
    implemented_backends,
)
from ._errors import BuildError, ConfigurationError, ToolchainMissingError
from ._manager import CompiledClass, CompiledFunction, Manager, initialize
from ._precompile import ImportFailure, Report, precompile

__all__ = [
    "DEFAULT_BACKEND",
    "DISABLE_ENV",
    "BackendNotAvailableError",
    "Behavior",
    "BuildError",
    "CompilationError",
    "CompiledClass",
    "CompiledFunction",
    "CompylrError",
    "ConfigurationError",
    "ImportFailure",
    "Manager",
    "Report",
    "Settings",
    "SourceSyntaxError",
    "ToolchainMissingError",
    "UnsupportedProgramError",
    "backend_names",
    "disabled_by_environment",
    "implemented_backends",
    "initialize",
    "precompile",
]
