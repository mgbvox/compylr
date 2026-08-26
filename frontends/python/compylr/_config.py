"""Settings, and how a per-function override resolves against the project's defaults."""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from typing import Final

from ._core import InvalidBehaviorError, behavior_axes, check_backend, check_behavior
from ._errors import ConfigurationError

__all__ = [
    "DEFAULT_BACKEND",
    "DISABLE_ENV",
    "Behavior",
    "Settings",
    "disabled_by_environment",
]

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

#: Python's user-facing names mapped onto the IR's neutral identifiers.
_BEHAVIOR_AXES: Final = {
    "overflow": "integer_overflow",
    "floor_div": "integer_division",
    "true_div": "exact_division",
    "modulo": "remainder",
    "index": "sequence_index",
    "text_len": "text_length",
}

#: Sentinel for "not specified here, inherit it".
#:
#: `None` cannot serve: `llm_assist=None` is indistinguishable from omitting it, and silently
#: treating an explicit `None` as "inherit" would hide a caller's mistake.
_INHERIT: Final = object()


@dataclass(frozen=True, slots=True, init=False)
class Behavior:
    """A partial choice of which language supplies each operation's meaning.

    ``None`` means inherit from the enclosing settings. Constructing settings resolves every
    field, first against Python's stance for project defaults and then against the project's
    resolved behavior for a member override.
    """

    overflow: str | None = None
    floor_div: str | None = None
    true_div: str | None = None
    modulo: str | None = None
    index: str | None = None
    text_len: str | None = None

    def __init__(
        self,
        *,
        overflow: str | None = None,
        floor_div: str | None = None,
        true_div: str | None = None,
        modulo: str | None = None,
        index: str | None = None,
        text_len: str | None = None,
        **unknown: str | None,
    ) -> None:
        if unknown:
            names = ", ".join(sorted(unknown))
            valid = " ".join(sorted(_BEHAVIOR_AXES))
            noun = "field" if len(unknown) == 1 else "fields"
            raise TypeError(f"unknown behavior {noun} {names}; valid fields: {valid}")

        object.__setattr__(self, "overflow", overflow)
        object.__setattr__(self, "floor_div", floor_div)
        object.__setattr__(self, "true_div", true_div)
        object.__setattr__(self, "modulo", modulo)
        object.__setattr__(self, "index", index)
        object.__setattr__(self, "text_len", text_len)

    @classmethod
    def from_language(cls, language: str) -> Behavior:
        """Return a behavior naming ``language`` on every axis."""
        return cls(
            overflow=language,
            floor_div=language,
            true_div=language,
            modulo=language,
            index=language,
            text_len=language,
        )

    def merge(self, override: str | Behavior) -> Behavior:
        """Apply a language or partial behavior over this behavior."""
        selected = self.from_language(override) if isinstance(override, str) else override
        return Behavior(
            overflow=selected.overflow if selected.overflow is not None else self.overflow,
            floor_div=selected.floor_div if selected.floor_div is not None else self.floor_div,
            true_div=selected.true_div if selected.true_div is not None else self.true_div,
            modulo=selected.modulo if selected.modulo is not None else self.modulo,
            index=selected.index if selected.index is not None else self.index,
            text_len=selected.text_len if selected.text_len is not None else self.text_len,
        )

    def to_core(self) -> dict[str, str]:
        """Spell the selected fields with the compiler's neutral axis identifiers."""
        return {
            axis: language
            for field_name, axis in _BEHAVIOR_AXES.items()
            if (language := getattr(self, field_name)) is not None
        }


def _check_selected_behavior(behavior: Behavior, backend: str) -> None:
    """Validate a partial behavior while retaining which field named a bad language."""
    for axis, language in behavior.to_core().items():
        try:
            check_behavior({axis: language}, backend)
        except InvalidBehaviorError as error:
            enhanced = InvalidBehaviorError(f"{axis}: {error}")
            enhanced.code = error.code
            raise enhanced from None


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


@dataclass(frozen=True, slots=True, init=False)
class Settings:
    """Resolved settings for one function, or the project's defaults."""

    backend: str = DEFAULT_BACKEND
    llm_assist: bool = False
    behavior: Behavior = field(default_factory=lambda: Behavior.from_language("python"))

    def __init__(
        self,
        backend: str = DEFAULT_BACKEND,
        llm_assist: bool = False,
        behavior: str | Behavior = "python",
    ) -> None:
        check_backend(backend)
        if isinstance(behavior, Behavior):
            _check_selected_behavior(behavior, backend)
        elif not isinstance(behavior, str):
            raise TypeError("behavior must be a language name or Behavior")
        resolved_behavior = Behavior.from_language("python").merge(behavior)
        object.__setattr__(self, "backend", backend)
        object.__setattr__(self, "llm_assist", llm_assist)
        object.__setattr__(self, "behavior", resolved_behavior)
        self.__post_init__()

    def __post_init__(self) -> None:
        # Validated on construction so a bad backend is reported by the decorator that named it,
        # rather than surfacing much later from a build.
        check_backend(self.backend)
        compiler_axes = set(behavior_axes())
        if set(_BEHAVIOR_AXES.values()) != compiler_axes:
            raise RuntimeError(
                "the Python behavior fields do not match the compiler axes: "
                f"Python maps {sorted(_BEHAVIOR_AXES.values())}, compiler reports "
                f"{sorted(compiler_axes)}"
            )
        check_behavior(self.behavior.to_core(), self.backend)
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
        behavior: str | Behavior | object = _INHERIT,
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
        if behavior is not _INHERIT:
            if not isinstance(behavior, (str, Behavior)):
                raise TypeError("behavior must be a language name or Behavior")
            if isinstance(behavior, Behavior):
                _check_selected_behavior(behavior, self.backend)
            changes["behavior"] = self.behavior.merge(behavior)
        if not changes:
            return self
        return replace(self, **changes)  # type: ignore[arg-type]


#: Re-exported so the decorator can spell "argument not given" the same way.
INHERIT: Final = _INHERIT
