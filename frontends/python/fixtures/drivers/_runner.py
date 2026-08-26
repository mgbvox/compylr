"""The driver format, the calls it declares, and the one canonical transcript.

A **driver** states which calls exercise one accepted fixture. It is *data*, not code: a module
whose only meaning is a ``CALLS`` list of literals, read here with :func:`ast.literal_eval` and
never executed. Two tiers consume the same declaration -- the boundary tier imports this module
and compares Python objects, and the translation tier asks Python to render the same calls as
JSON and compares that against what generated Rust printed. A driver written as free Python
would be readable by the first tier only, so the second would need its calls stated a second
time, and two statements of the same calls drift the first time someone edits one.

A driver carries **no expected values**. What a call should answer is what CPython answers when
the same driver runs against the unmodified fixture, so there is nothing for a person to type
incorrectly.

The transcript is JSON with mapping keys sorted and sets rendered as sorted arrays, because the
accepted subset promises neither mapping nor set iteration order -- a rendering that preserved
insertion order would make the suite flaky rather than make the compiler right.
"""

from __future__ import annotations

import ast
import json
import math
from collections.abc import Iterable
from pathlib import Path
from types import SimpleNamespace
from typing import Any, get_args, get_origin, get_type_hints

#: How far two floats may differ and still be the same answer.
#:
#: The compiled and interpreted paths can differ in the last bit, and
#: ``demo/src/algorithms/__main__.py`` already made this call for the same reason; the two places
#: agree by test rather than by coincidence.
FLOAT_RELATIVE_TOLERANCE = 1e-9

#: Significant digits the renderer keeps, which is what the tolerance implies.
#:
#: Rendering coarser than the tolerance is what lets the translation tier compare *text* and still
#: honour the tolerance: two values that compare equal under it render to the same string.
FLOAT_SIGNIFICANT_DIGITS = round(-math.log10(FLOAT_RELATIVE_TOLERANCE))

_ALLOWED_KEYS = {"call", "new", "args", "methods"}


class DriverError(Exception):
    """A driver that cannot be read, or a call it declares that cannot be made."""


def load_calls(path: Path | str) -> list[dict[str, Any]]:
    """Read a driver's ``CALLS`` declaration as literal data.

    The module is parsed, never imported, so a driver cannot run anything.
    """
    path = Path(path)
    name = path.name
    try:
        tree = ast.parse(path.read_text(), filename=str(path))
    except SyntaxError as error:  # pragma: no cover - a driver that will not parse
        raise DriverError(f"{name}: is not valid Python: {error}") from error

    node = _calls_node(tree, name)
    try:
        calls = ast.literal_eval(node)
    except ValueError as error:
        raise DriverError(
            f"{name}: CALLS must be literal data readable without executing the module, "
            f"but it is not: {error}"
        ) from error

    _validate(calls, name)
    return calls


def _calls_node(tree: ast.Module, name: str) -> ast.expr:
    for statement in tree.body:
        if isinstance(statement, ast.Assign):
            targets = [t.id for t in statement.targets if isinstance(t, ast.Name)]
            if "CALLS" in targets:
                return statement.value
        if (
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id == "CALLS"
            and statement.value is not None
        ):
            return statement.value
    raise DriverError(f"{name}: declares no CALLS list; every driver must declare one")


def _validate(calls: object, name: str) -> None:
    if not isinstance(calls, list):
        raise DriverError(f"{name}: CALLS must be a list of calls, not {type(calls).__name__}")
    if not calls:
        raise DriverError(f"{name}: CALLS is empty; a driver that calls nothing proves nothing")
    for index, entry in enumerate(calls):
        _validate_entry(entry, name, f"CALLS[{index}]")


def _validate_entry(entry: object, name: str, where: str, *, argument: bool = False) -> None:
    if not isinstance(entry, dict):
        raise DriverError(
            f"{name}: {where} must be a mapping naming a call, not {type(entry).__name__}"
        )
    if "call" in entry and "new" in entry:
        raise DriverError(f"{name}: {where} names both a function and a class; it must name one")
    if "call" not in entry and "new" not in entry:
        raise DriverError(
            f"{name}: {where} names neither a function to call nor a class to construct"
        )
    extra = set(entry) - _ALLOWED_KEYS
    if extra:
        raise DriverError(f"{name}: {where} has extra keys {sorted(extra)}")

    member = entry.get("call", entry.get("new"))
    if not isinstance(member, str) or not member:
        raise DriverError(f"{name}: {where} must name its member as a string")

    args = entry.get("args", [])
    if not isinstance(args, list):
        raise DriverError(f"{name}: {where} arguments must be a list, not {type(args).__name__}")
    for supplied in args:
        # A mapping is data unless it names a class: fixtures take dict arguments, and reading
        # every one of them as a call would make `lookup({"a": 1}, "a")` unstatable.
        if isinstance(supplied, dict) and "new" in supplied:
            _validate_entry(supplied, name, f"{where} argument", argument=True)

    methods = entry.get("methods", [])
    if not isinstance(methods, list):
        raise DriverError(f"{name}: {where} methods must be a list, not {type(methods).__name__}")
    for method in methods:
        if not isinstance(method, (list, tuple)) or len(method) != 2:
            raise DriverError(
                f"{name}: {where} each method must be a name and its arguments, got {method!r}"
            )
        method_name, method_args = method
        if not isinstance(method_name, str):
            raise DriverError(f"{name}: {where} method names must be strings, got {method_name!r}")
        if not isinstance(method_args, list):
            raise DriverError(
                f"{name}: {where} method {method_name} arguments must be a list, "
                f"not {type(method_args).__name__}"
            )
    # An instance built to be *passed* is observed through the call that receives it; one
    # built as a call in its own right is observed only through its methods, so it must have some.
    if "new" in entry and not methods and not argument:
        raise DriverError(
            f"{name}: {where} constructs {member} but calls no method on it, so nothing about the "
            f"instance is observed"
        )


def encode_calls(calls: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Re-express a driver's calls so JSON can carry them without losing a type.

    The translation tier reads drivers through JSON, and JSON has one sequence type and only
    string keys -- so a set would arrive as a list, a tuple as a list, and an integer mapping key
    as its spelling. The generated harness has to build a `HashSet` where the driver wrote a set,
    so the distinction has to survive the trip. Values that JSON already represents exactly are
    left alone; the rest are tagged.
    """
    return [_encode_entry(entry) for entry in calls]


def _encode_entry(entry: dict[str, Any]) -> dict[str, Any]:
    encoded: dict[str, Any] = {}
    if "call" in entry:
        encoded["call"] = entry["call"]
    else:
        encoded["new"] = entry["new"]
    encoded["args"] = [_encode_argument(argument) for argument in entry.get("args", [])]
    encoded["methods"] = [
        [name, [_encode_argument(argument) for argument in args]]
        for name, args in entry.get("methods", [])
    ]
    return encoded


def _encode_argument(argument: Any) -> Any:
    # A mapping naming a class is a call, not data -- the same rule the validator applies.
    if isinstance(argument, dict) and "new" in argument:
        return _encode_entry(argument)
    return encode_value(argument)


def encode_value(value: Any) -> Any:
    """Re-express a value so JSON can carry it without losing a type.

    Used for arguments on the way to the translation tier, and for results on the way back from
    the separate process that produces the interpreted side.
    """
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, list):
        return [encode_value(item) for item in value]
    if isinstance(value, tuple):
        return {"$tuple": [encode_value(item) for item in value]}
    if isinstance(value, (set, frozenset)):
        return {"$set": [encode_value(item) for item in sorted(value)]}
    if isinstance(value, dict):
        return {"$dict": [[encode_value(k), encode_value(v)] for k, v in value.items()]}
    raise DriverError(f"no JSON encoding for a {type(value).__name__}")


def members_named(calls: list[dict[str, Any]]) -> set[str]:
    """Every fixture member a driver reaches, including classes built to pass as arguments."""
    found: set[str] = set()

    def walk(entry: dict[str, Any]) -> None:
        found.add(entry.get("call", entry.get("new", "")))
        for argument in entry.get("args", []):
            if isinstance(argument, dict) and "new" in argument:
                walk(argument)

    for entry in calls:
        walk(entry)
    return found


def _declared_return(member: Any) -> Any:
    """The return type a member declares, or ``None`` when it declares nothing readable.

    A compiled member is backed by Rust and carries no annotations, in which case there is
    nothing to coerce to and the value is already at its declared type.
    """
    try:
        annotations = get_type_hints(member)
    except (TypeError, NameError, AttributeError):
        # A compiled member is backed by Rust and has no annotations to resolve.
        return None
    declared = annotations.get("return")
    # A spelling that would not resolve is not a type this can coerce to, and guessing is worse
    # than leaving the value alone.
    return None if isinstance(declared, str) else declared


def coerce_to_declared(value: Any, declared: Any) -> Any:
    """Bring a value to the type its signature declares.

    Python's annotations do not coerce: `def widen(n: int) -> float: return n` answers the integer
    3, while the same function translated answers 3.0. The two are the same answer -- the
    transcript is rendered from the *declared* type on both sides, or the tier would report a
    difference in Python's runtime typing as a difference in the compiler.
    """
    if declared is None or declared is type(None):
        return value
    if declared is float and isinstance(value, int) and not isinstance(value, bool):
        return float(value)

    origin, args = get_origin(declared), get_args(declared)
    if origin in (list, set, frozenset) and args and isinstance(value, (list, set, frozenset)):
        items = [coerce_to_declared(item, args[0]) for item in value]
        return items if origin is list else set(items)
    if origin is dict and len(args) == 2 and isinstance(value, dict):
        return {
            coerce_to_declared(k, args[0]): coerce_to_declared(v, args[1]) for k, v in value.items()
        }
    if origin is tuple and args and isinstance(value, tuple):
        return tuple(coerce_to_declared(item, arg) for item, arg in zip(value, args, strict=False))
    return value


def run_calls(calls: list[dict[str, Any]], module: Any) -> list[Any]:
    """Invoke a driver's calls in order against ``module`` and return their *values*.

    Values rather than text -- D2. The boundary tier already holds both answers as Python
    objects, so it compares mappings and sets by content and floats within the tolerance;
    rendering them first would invent an ordering problem the comparison does not have.
    """
    return [_run_entry(entry, module) for entry in calls]


def _run_entry(entry: dict[str, Any], module: Any) -> Any:
    args = [_realise(argument, module) for argument in entry.get("args", [])]
    methods = entry.get("methods", [])

    if "new" in entry:
        instance = _member(module, entry["new"])(*args)
        return [_call_method(instance, name, method_args, module) for name, method_args in methods]

    member = _member(module, entry["call"])
    result = member(*args)
    if not methods:
        return coerce_to_declared(result, _declared_return(member))
    return [_call_method(result, name, method_args, module) for name, method_args in methods]


def _call_method(instance: Any, name: str, args: list[Any], module: Any) -> Any:
    method = getattr(instance, name, None)
    if method is None:
        raise DriverError(f"{type(instance).__name__} has no method {name}")
    result = method(*[_realise(argument, module) for argument in args])
    declared = _declared_return(getattr(type(instance), name, None))
    return coerce_to_declared(result, declared)


def _realise(argument: Any, module: Any) -> Any:
    """Turn a declared argument into a value, constructing an instance where one is named."""
    if isinstance(argument, dict) and "new" in argument:
        built = [_realise(inner, module) for inner in argument.get("args", [])]
        return _member(module, argument["new"])(*built)
    return argument


def _member(module: Any, name: str) -> Any:
    member = getattr(module, name, None)
    if member is None:
        raise DriverError(f"{name} is not defined by the fixture under test")
    return member


def render_transcript(results: list[Any]) -> str:
    """One canonical line per call."""
    return "\n".join(render_value(result) for result in results)


def render_value(value: Any) -> str:
    """Render one value as canonical JSON text.

    Mapping keys are sorted and sets become sorted arrays, so nothing here depends on an
    iteration order the language does not promise. Floats take one fixed spelling, which settles
    the differences that would otherwise be noise between two languages: ``True`` against
    ``true``, ``'a'`` against ``"a"``, and one runtime's idea of a float's digits against
    another's.
    """
    # bool before int: `isinstance(True, int)` is true, and a bool must not render as 1.
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return render_float(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=True)
    if isinstance(value, (list, tuple)):
        return "[" + ",".join(render_value(item) for item in value) + "]"
    if isinstance(value, (set, frozenset)):
        return "[" + ",".join(render_value(item) for item in sorted(value)) + "]"
    if isinstance(value, dict):
        pairs = sorted(value.items(), key=lambda pair: _key_order(pair[0]))
        return "{" + ",".join(f"{_render_key(k)}:{render_value(v)}" for k, v in pairs) + "}"
    raise DriverError(f"no canonical rendering for a {type(value).__name__}")


def render_float(value: float) -> str:
    """One fixed spelling: a mantissa of fixed width and a signed exponent with no padding.

    Chosen because both languages can produce it identically. Python pads an exponent to two
    digits and Rust does not, so the exponent is re-rendered from its integer value on both
    sides rather than taken from either one's default.
    """
    if math.isnan(value):
        return '"NaN"'
    if math.isinf(value):
        return '"Infinity"' if value > 0 else '"-Infinity"'
    mantissa, _, exponent = f"{value:.{FLOAT_SIGNIFICANT_DIGITS - 1}e}".partition("e")
    return f"{mantissa}e{int(exponent):+d}"


def _key_order(key: Any) -> tuple[int, Any]:
    """Sort keys by their own type's order, never by their spelling."""
    if isinstance(key, bool):
        return (0, int(key))
    if isinstance(key, (int, float)):
        return (0, key)
    return (1, str(key))


def _render_key(key: Any) -> str:
    """A JSON object key is always a string, whatever the mapping's key type is."""
    if isinstance(key, str):
        return json.dumps(key, ensure_ascii=True)
    if isinstance(key, bool):
        return '"true"' if key else '"false"'
    if isinstance(key, float):
        return json.dumps(render_float(key).strip('"'))
    return json.dumps(str(key))


def values_agree(left: Any, right: Any) -> bool:
    """Whether two results are the same answer.

    ``==`` for everything but floats, which agree within
    :data:`FLOAT_RELATIVE_TOLERANCE`. Mappings and sets compare by content, so iteration order
    never enters into it -- asserting on that order is what would make this flaky.
    """
    if isinstance(left, bool) or isinstance(right, bool):
        return left is right
    if isinstance(left, float) or isinstance(right, float):
        if not isinstance(left, (int, float)) or not isinstance(right, (int, float)):
            return False
        return math.isclose(left, right, rel_tol=FLOAT_RELATIVE_TOLERANCE)
    if isinstance(left, (list, tuple)) and isinstance(right, (list, tuple)):
        return len(left) == len(right) and all(
            values_agree(a, b) for a, b in zip(left, right, strict=True)
        )
    if isinstance(left, dict) and isinstance(right, dict):
        return set(left) == set(right) and all(values_agree(left[k], right[k]) for k in left)
    if isinstance(left, (set, frozenset)) and isinstance(right, (set, frozenset)):
        return left == right
    if type(left) is not type(right):
        return False
    return bool(left == right)


#: Fixtures whose names begin with this only mean anything together: one calls into the other.
CROSS_SOURCE_PREFIX = "cross_source_"


def group_for(stem: str, every_stem: Iterable[str]) -> list[str]:
    """The fixtures that must be present for ``stem`` to resolve.

    Stated once, here, because both tiers need the same answer: a call across two sources
    resolves only when both are in the same unit, and the translation tier groups its units the
    way `emit_quality.rs` already does.
    """
    if stem.startswith(CROSS_SOURCE_PREFIX):
        return sorted(s for s in every_stem if s.startswith(CROSS_SOURCE_PREFIX))
    return [stem]


def fixture_namespace(paths: Iterable[Path]) -> SimpleNamespace:
    """Run fixture sources into one namespace, the way one unit shares one namespace.

    This is the oracle side: plain CPython running the fixture's own source, with nothing about
    it depending on the compiler being correct.
    """
    namespace: dict[str, Any] = {}
    for path in paths:
        exec(compile(path.read_text(), str(path), "exec"), namespace)  # noqa: S102
    # `__builtins__` stays. Annotations are evaluated lazily, against the defining module's
    # globals -- remove it and `-> float` can no longer resolve `float`, so every annotation
    # degrades to its own spelling and the declared type silently stops being knowable.
    return SimpleNamespace(**namespace)


def interpreted_results(accepted_dir: Path, drivers_dir: Path, stem: str) -> list[Any]:
    """What CPython answers for one fixture's driver."""
    stems = [p.stem for p in accepted_dir.glob("*.py")]
    sources = [accepted_dir / f"{name}.py" for name in group_for(stem, stems)]
    calls = load_calls(drivers_dir / f"{stem}.py")
    return run_calls(calls, fixture_namespace(sources))


def decode_value(encoded: Any) -> Any:
    """Undo :func:`encode_calls`'s tagging, so a value can round-trip through JSON."""
    if isinstance(encoded, list):
        return [decode_value(item) for item in encoded]
    if isinstance(encoded, dict):
        if "$set" in encoded:
            return {decode_value(item) for item in encoded["$set"]}
        if "$tuple" in encoded:
            return tuple(decode_value(item) for item in encoded["$tuple"])
        if "$dict" in encoded:
            return {decode_value(k): decode_value(v) for k, v in encoded["$dict"]}
    return encoded


def render_encoded(encoded: Any) -> str:
    """Render a tagged value the way a transcript renders it.

    The translation tier's renderer is written twice -- once here and once in Rust -- and a
    renderer written twice is a renderer written wrong. This is the entry point the mirror test
    uses to hold the two together.
    """
    return render_value(decode_value(encoded))
