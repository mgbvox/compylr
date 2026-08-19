"""compylr — transpiles a strict, fully annotated Python subset to Rust.

    import compylr

    c = compylr.initialize(backend="rust", llm_assist=False)

    @c.compyle
    def add(a: int, b: int) -> int:
        return a + b

The first call to a marked function compiles every marked function in the project into one shared
Rust extension and swaps the compiled implementation in. Later runs reuse it, keyed on a
fingerprint of the IR rather than of the source text, so comments and reformatting do not trigger
a rebuild.

Compiling requires a Rust toolchain and maturin on the machine running the project; `uv add
compylr` on its own installs the compiler, not the ability to build what it generates.
"""

from __future__ import annotations

from ._config import DEFAULT_BACKEND, DISABLE_ENV, Settings, disabled_by_environment
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
