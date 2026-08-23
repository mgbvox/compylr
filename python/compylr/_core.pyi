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
    #: Optimization passes that ran, in order. Recorded in build state, so that an artifact
    #: produced by a different set is rebuilt rather than reused.
    passes: list[str]

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

class InvalidBehaviorError(CompylrError):
    """A behavior named something that is not one of this compilation's two languages.

    Also raised for an axis that does not exist. Branch on `code` rather than on the message:
    the three cases read very differently to a user and the prose is free to be reworded.
    """

    #: One of `unknown_language`, `language_not_in_pair`, or `unknown_axis`.
    code: str

def compile_unit(
    sources: list[tuple[str, dict[str, str]]],
    backend: str = "rust",
    frontend: str = "python",
) -> CompiledUnit:
    """Compile source texts into a target artifact, each under its own behavior.

    Both ends of the pipeline are named and defaulted. `frontend` is not a knob anybody needs
    today -- there is one source language -- but hardcoding it would have made the Python host the
    only caller that could not ask for another, which is the asymmetry the workspace exists to
    avoid.

    Each source is paired with a mapping from behavior axis to language name, using the axis
    identifiers `behavior_axes` reports. An empty mapping means every axis inherits, which
    resolves to the source language's stance -- so a caller that never mentions behavior gets
    what it got before the setting existed.

    Paired rather than supplied as a second list, because the behavior is a property of the
    member: two lists indexed together are two lists that can come apart, and the failure that
    produces is a function compiled under its neighbour's meanings.
    """

def validate_source(source: str) -> list[str]:
    """Check one source against the supported subset, returning its function names."""

def backend_names() -> list[str]:
    """Every backend name compylr recognizes, implemented or not."""

def implemented_backends() -> list[str]:
    """Every backend name that can compile today."""

def check_backend(name: str) -> None:
    """Raise unless `name` is a backend that can compile today."""

def check_behavior(
    axes: dict[str, str], backend: str = "rust", frontend: str = "python"
) -> None:
    """Raise unless every axis and language named is valid for this pair.

    Nothing is parsed and no target source is generated. This is what lets the decorator reject a
    bad behavior as it runs, rather than at a build reached much later.
    """

def behavior_axes() -> list[str]:
    """Every behavior axis, by its stable identifier.

    Exposed so the Python surface can check its own field names against the compiler's rather
    than carrying a second list that drifts.
    """
