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

from ._config import DEFAULT_BACKEND, Settings
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
from ._manager import CompiledFunction, Manager, initialize

__all__ = [
    "DEFAULT_BACKEND",
    "BackendNotAvailableError",
    "BuildError",
    "CompilationError",
    "CompiledFunction",
    "CompylrError",
    "ConfigurationError",
    "Manager",
    "Settings",
    "SourceSyntaxError",
    "ToolchainMissingError",
    "UnsupportedProgramError",
    "backend_names",
    "implemented_backends",
    "initialize",
]
