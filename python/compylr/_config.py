"""Settings, and how a per-function override resolves against the project's defaults."""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Final

from ._core import check_backend
from ._errors import ConfigurationError

__all__ = ["DEFAULT_BACKEND", "Settings"]

#: The backend used when none is named.
DEFAULT_BACKEND: Final = "rust"

#: Sentinel for "not specified here, inherit it".
#:
#: `None` cannot serve: `llm_assist=None` is indistinguishable from omitting it, and silently
#: treating an explicit `None` as "inherit" would hide a caller's mistake.
_INHERIT: Final = object()


@dataclass(frozen=True, slots=True)
class Settings:
    """Resolved settings for one function, or the project's defaults."""

    backend: str = DEFAULT_BACKEND
    llm_assist: bool = False

    def __post_init__(self) -> None:
        # Validated on construction so a bad backend is reported by the decorator that named it,
        # rather than surfacing much later from a build.
        check_backend(self.backend)
        if self.llm_assist:
            raise ConfigurationError(
                "llm_assist is not implemented yet. It is reserved for a mode that pipes source "
                "to a configurable backend agent to assist translation into the IR and into the "
                "target backend; the setting is accepted now so enabling it later will not "
                "require an API change."
            )

    def override(
        self,
        backend: str | object = _INHERIT,
        llm_assist: bool | object = _INHERIT,
    ) -> Settings:
        """Return these settings with the named fields replaced.

        Anything not named is inherited, which is what makes `@c.compyle(llm_assist=...)` leave the
        backend alone rather than silently resetting it to the default.
        """
        changes: dict[str, object] = {}
        if backend is not _INHERIT:
            changes["backend"] = backend
        if llm_assist is not _INHERIT:
            changes["llm_assist"] = llm_assist
        if not changes:
            return self
        return replace(self, **changes)  # type: ignore[arg-type]


#: Re-exported so the decorator can spell "argument not given" the same way.
INHERIT: Final = _INHERIT
