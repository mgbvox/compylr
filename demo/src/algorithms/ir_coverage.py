"""What this package exercises, read off the IR rather than claimed in prose.

"Showcases everything compylr can do" is the kind of sentence that is true when it is written and
quietly false a release later. So it is not a sentence here: `compylr` writes the IR of every
build to `.compylr/ir/unit.json`, and this module walks that file and reports which of the IR's
statement forms, expression forms, types, and operators actually appear — and which member of the
package is the first to use each.

`tests/test_coverage.py` turns the report into an assertion, so a form that stops being covered
fails the demo's own suite. The repository's `tests/demo_coverage.rs` closes the other half: it
reads the IR's enum definitions and fails if a form is added to the compiler that the tables
below do not know about, which is the way this file would otherwise go stale without anyone
noticing.

Nothing here is compiled. It reads JSON, which is not what compylr is for.
"""

from __future__ import annotations

import json
from collections.abc import Iterator
from pathlib import Path
from typing import Any

__all__ = ["TABLES", "Coverage", "load_artifact", "measure"]

#: Every statement form in the IR, as it is tagged in the artifact.
STATEMENTS = (
    "Return",
    "ReturnUnit",
    "Bind",
    "Assign",
    "Effect",
    "SetAttr",
    "SetItem",
    "Append",
    "If",
    "While",
    "For",
    "Break",
    "Continue",
)

#: Every expression form.
EXPRESSIONS = (
    "Literal",
    "Name",
    "Neg",
    "ToFloat",
    "Binary",
    "ListLit",
    "DictLit",
    "SetLit",
    "TupleLit",
    "TupleIndex",
    "Attribute",
    "Construct",
    "MethodCall",
    "Contains",
    "Not",
    "Subscript",
    "Len",
    "Range",
    "Call",
)

#: Every type. `Unit` is a return type only — it is what `-> None` becomes.
TYPES = (
    "Int",
    "Float",
    "Bool",
    "Str",
    "Unit",
    "List",
    "Dict",
    "Set",
    "Tuple",
    "Instance",
)

#: Every binary operator.
#:
#: `Div` and `Rem` are single variants carrying a *mode* — the rounding direction and the sign
#: convention a frontend declared — so covering the variant is not the same as covering both
#: readings. `MODES` below is what checks the rest.
OPERATORS = (
    "Add",
    "Sub",
    "Mul",
    "Div",
    "Rem",
    "Eq",
    "NotEq",
    "Lt",
    "LtE",
    "Gt",
    "GtE",
)

#: The declared semantics that a Python program can actually produce.
#:
#: Python's `/` is exact and its `//` is integer division rounding toward negative infinity, so
#: both `DivMode`s are reachable. The other three declarations — the remainder's sign, the index
#: origin, and the units a string's length is counted in — have exactly one Python reading each,
#: so a Python program cannot exercise the alternatives and the compiler's own conformance corpus
#: is what covers them. That is the whole reason that corpus is authored as IR.
MODES = ("Exact", "Integer")

#: The four tables, in the order the report prints them.
TABLES: dict[str, tuple[str, ...]] = {
    "statements": STATEMENTS,
    "expressions": EXPRESSIONS,
    "types": TYPES,
    "operators": OPERATORS,
}


def load_artifact(path: Path) -> dict[str, Any]:
    """The IR artifact, as written by the last build."""
    artifact: dict[str, Any] = json.loads(path.read_text())
    return artifact


def _tags(node: Any) -> Iterator[str]:
    """Every serde tag and bare-string variant under `node`.

    The artifact's enums are externally tagged, so a form appears either as an object with one
    key — `{"If": {...}}` — or, when it carries no data, as the bare string `"Break"`. Walking
    for both is what makes one traversal answer for statements, expressions, types, and
    operators alike.
    """
    if isinstance(node, dict):
        for key, value in node.items():
            yield key
            yield from _tags(value)
    elif isinstance(node, list):
        for value in node:
            yield from _tags(value)
    elif isinstance(node, str):
        yield node


def _members(artifact: dict[str, Any]) -> Iterator[tuple[str, Any]]:
    """Each top-level member of the unit, as (name, subtree)."""
    for function in artifact.get("functions", []):
        yield function["name"], function
    for klass in artifact.get("classes", []):
        yield klass["name"], klass


class Coverage:
    """Which IR forms a unit exercises, and which member reaches each one first."""

    def __init__(self, artifact: dict[str, Any]) -> None:
        self.first_use: dict[str, str] = {}
        for name, subtree in sorted(_members(artifact)):
            for tag in _tags(subtree):
                self.first_use.setdefault(tag, name)
        self.modes = tuple(mode for mode in MODES if mode in self.first_use)

    def covered(self, table: str) -> tuple[str, ...]:
        """The forms in `table` that appear."""
        return tuple(form for form in TABLES[table] if form in self.first_use)

    def missing(self, table: str) -> tuple[str, ...]:
        """The forms in `table` that do not."""
        return tuple(form for form in TABLES[table] if form not in self.first_use)

    def gaps(self) -> dict[str, tuple[str, ...]]:
        """Every table that is not fully covered, and what it is short of."""
        found = {table: self.missing(table) for table in TABLES}
        short = {table: forms for table, forms in found.items() if forms}
        if len(self.modes) != len(MODES):
            short["division modes"] = tuple(m for m in MODES if m not in self.modes)
        return short

    def report(self) -> str:
        """The coverage table, as text."""
        lines: list[str] = []
        for table, forms in TABLES.items():
            covered = self.covered(table)
            lines.append(f"{table} — {len(covered)}/{len(forms)}")
            for form in forms:
                where = self.first_use.get(form)
                mark = "  " if where else "  MISSING "
                lines.append(f"    {form:<12}{mark}{where or ''}")
            lines.append("")
        modes = ", ".join(self.modes) if self.modes else "none"
        lines.append(f"division modes — {len(self.modes)}/{len(MODES)}: {modes}")
        return "\n".join(lines)


def measure(path: Path) -> Coverage:
    """Coverage of the IR artifact at `path`."""
    return Coverage(load_artifact(path))
