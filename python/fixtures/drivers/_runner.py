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
from pathlib import Path
from typing import Any

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
        if isinstance(supplied, dict):
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


def members_named(calls: list[dict[str, Any]]) -> set[str]:
    """Every fixture member a driver reaches, including classes built to pass as arguments."""
    found: set[str] = set()

    def walk(entry: dict[str, Any]) -> None:
        found.add(entry.get("call", entry.get("new", "")))
        for argument in entry.get("args", []):
            if isinstance(argument, dict):
                walk(argument)

    for entry in calls:
        walk(entry)
    return found


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

    result = _member(module, entry["call"])(*args)
    if not methods:
        return result
    return [_call_method(result, name, method_args, module) for name, method_args in methods]


def _call_method(instance: Any, name: str, args: list[Any], module: Any) -> Any:
    method = getattr(instance, name, None)
    if method is None:
        raise DriverError(f"{type(instance).__name__} has no method {name}")
    return method(*[_realise(argument, module) for argument in args])


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
