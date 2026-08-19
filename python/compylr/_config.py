"""Settings, and how a per-function override resolves against the project's defaults."""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Final

from ._core import check_backend
from ._errors import ConfigurationError

__all__ = ["DEFAULT_BACKEND", "DISABLE_ENV", "Settings", "disabled_by_environment"]

#: Environment variable that turns compilation off for a whole process.
#:
#: Set it to run a project as ordinary interpreted Python: marked functions and classes are left
#: exactly as written, nothing is validated, and no build is attempted. That makes it the fastest
#: way to answer "is this compylr, or is it my code?" — and the only honest way to time the
#: interpreted side of a comparison, since a function's calls to *other* marked functions resolve
#: through module globals and would otherwise still reach compiled code.
DISABLE_ENV: Final = "COMPYLR_DISABLE"

_TRUTHY: Final = frozenset({"1", "true", "yes", "on"})
_FALSEY: Final = frozenset({"0", "false", "no", "off", ""})

#: The backend used when none is named.
DEFAULT_BACKEND: Final = "rust"

#: Sentinel for "not specified here, inherit it".
#:
#: `None` cannot serve: `llm_assist=None` is indistinguishable from omitting it, and silently
#: treating an explicit `None` as "inherit" would hide a caller's mistake.
_INHERIT: Final = object()


def disabled_by_environment() -> bool:
    """Whether the environment asks for compilation to be turned off.

    An unrecognised value is an error rather than a quiet "no". `COMPYLR_DISABLE=maybe` silently
    meaning "enabled" is exactly the kind of wrongness that is discovered much later, and the whole
    point of the switch is to be sure which mode you are in.
    """
    import os

    raw = os.environ.get(DISABLE_ENV)
    if raw is None:
        return False
    value = raw.strip().lower()
    if value in _TRUTHY:
        return True
    if value in _FALSEY:
        return False
    raise ConfigurationError(
        f"{DISABLE_ENV}={raw!r} is not a recognised value; use one of "
        f"{sorted(_TRUTHY)} to disable or {sorted(_FALSEY - {''})} to enable"
    )


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
