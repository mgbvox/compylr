"""Failures that arise on the Python side of the pipeline.

The compilation failures live in the native module, because that is where they are detected. These
are the ones that only exist once compylr starts touching a real machine: building, installing, and
being configured.

They share `CompylrError` as a base so a caller can catch everything compylr raises with one
clause, which is the whole point of having a base at all.
"""

from __future__ import annotations

from ._core import CompylrError

__all__ = [
    "BuildError",
    "ConfigurationError",
    "ToolchainMissingError",
]


class BuildError(CompylrError):
    """The generated crate failed to build or install.

    Carries the toolchain's own output. A build failure that only said "build failed" would leave
    a user with nothing to act on, and the compiler's diagnostics are the actionable part.
    """

    def __init__(self, message: str, output: str = "") -> None:
        super().__init__(message if not output else f"{message}\n\n{output}")
        self.output = output


class ToolchainMissingError(BuildError):
    """A required build tool is not installed.

    Distinct from a build failure because the fix is completely different: one means the generated
    code is wrong, the other means the machine cannot build anything yet.
    """

    def __init__(self, tool: str, hint: str) -> None:
        super().__init__(f"{tool} is required to compile, but was not found. {hint}")
        self.tool = tool


class ConfigurationError(CompylrError):
    """compylr was asked for something its configuration cannot express."""
