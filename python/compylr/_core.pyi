"""Type declarations for the compiled half of compylr.

`_core` is a native module, so mypy has no source to read. This stub is the declaration of what
the Rust side promises; if the two drift, the Python package type-checks against a lie.
"""

class CompiledUnit:
    """Everything a successful compilation produces."""

    #: Generated files, keyed by path relative to the crate root.
    target_sources: dict[str, str]
    ir_artifact: str
    fingerprint: str
    module_name: str
    manifest: str
    function_names: list[str]

class CompylrError(Exception):
    """Base class for every compylr failure."""

class CompilationError(CompylrError):
    """A program could not be compiled."""

    line: int
    column: int
    #: Stable identifier for the category, or None for a syntax error. Branch on this rather
    #: than on the message, which is prose.
    code: str | None

class SourceSyntaxError(CompilationError):
    """The source is not valid Python."""

class UnsupportedProgramError(CompilationError):
    """Valid Python, but outside the supported subset."""

class BackendNotAvailableError(CompylrError):
    """The requested backend is unknown, or reserved but not implemented."""

def compile_unit(sources: list[str], backend: str = "rust") -> CompiledUnit:
    """Compile source texts into a target artifact."""

def validate_source(source: str) -> list[str]:
    """Check one source against the supported subset, returning its function names."""

def backend_names() -> list[str]:
    """Every backend name compylr recognizes, implemented or not."""

def implemented_backends() -> list[str]:
    """Every backend name that can compile today."""

def check_backend(name: str) -> None:
    """Raise unless `name` is a backend that can compile today."""
