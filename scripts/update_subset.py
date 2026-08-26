#!/usr/bin/env python3
"""Generate the README's subset matrix from the corpus, and from evidence that it works.

    python scripts/update_subset.py            # rewrite the table
    python scripts/update_subset.py --check    # verify it is current; measures nothing

What the README claims compylr accepts is **counted**, not remembered. The rule that makes the
claim worth reading is `py2many`'s, in `LANGUAGES.md`: a construct is reported as accepted only
because a fixture exercising it translated, built, ran, and **agreed with CPython**. A construct
with no passing fixture does not appear, so the documentation cannot overstate the implementation.

Two inputs, and both are evidence rather than assertion:

* **What each fixture exercises** comes from its IR, emitted by the compiler.
* **Which fixtures agree** comes from running the translation tier. If it does not pass, nothing
  is claimed at all.

This is a sibling of `update_benchmarks.py` rather than part of it. They are different jobs with
the same output mechanism -- benchmarks measure and take minutes, this counts -- and folding the
second into the first would make a documentation check depend on a benchmark run.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from _regions import MarkerError, Region, find_region, replace_region

REPO = Path(__file__).resolve().parents[1]
ACCEPTED = REPO / "python/fixtures/accepted"

#: The generated block this script owns.
MATRIX = Region("matrix", REPO / "README.md", prefix="subset")

#: How the categories are titled, in the order the table prints them.
KINDS = {
    "statements": "statement",
    "expressions": "expression",
    "types": "type",
    "operators": "operator",
}


def _tables() -> dict[str, tuple[str, ...]]:
    """The IR forms, by category.

    Read from the demo's `ir_coverage.py` rather than restated here. That is the one place these
    lists live, and `crates/compylr-host-python/tests/demo_coverage.rs` reads the IR's own enum
    definitions and fails when a form is added that they do not list -- so they are guarded
    against the compiler. A second copy would be a second thing to keep in step.
    """
    path = REPO / "demo/demo-python-rust/src/algorithms/ir_coverage.py"
    spec = importlib.util.spec_from_file_location("_ir_coverage", path)
    if spec is None or spec.loader is None:  # pragma: no cover - only if the demo moves
        raise RuntimeError(f"cannot read the IR form tables from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    tables: dict[str, tuple[str, ...]] = module.TABLES
    return tables


def tags(node: object) -> set[str]:
    """Every serde tag and bare-string variant under `node`.

    The artifact's enums are externally tagged, so a form appears either as an object with one key
    -- `{"If": {...}}` -- or, carrying no data, as the bare string `"Break"`. Walking for both is
    what makes one traversal answer for statements, expressions, types, and operators alike.
    """
    found: set[str] = set()
    if isinstance(node, dict):
        for key, value in node.items():
            found.add(key)
            found |= tags(value)
    elif isinstance(node, list):
        for value in node:
            found |= tags(value)
    elif isinstance(node, str):
        found.add(node)
    return found


def fixture_stems() -> list[str]:
    """The accepted corpus, read from the directory rather than listed."""
    return sorted(path.stem for path in ACCEPTED.glob("*.py"))


def groups() -> list[list[str]]:
    """The corpus, grouped so that cross-source calls resolve.

    The same grouping both differential tiers use: a call across two sources is only well formed
    once both are in one unit, so `cross_source_caller.py` has no IR of its own.
    """
    stems = fixture_stems()
    cross = [stem for stem in stems if stem.startswith("cross_source_")]
    singles = [[stem] for stem in stems if not stem.startswith("cross_source_")]
    return singles + ([cross] if cross else [])


def emit_ir(stems: list[str]) -> dict[str, object]:
    """The IR the compiler produces for one group of fixtures.

    The CLI takes a single file, so a group is handed over as one source. Every fixture is
    top-level definitions with no imports, so concatenating them is the same unit the tiers build.
    """
    if len(stems) == 1:
        source_path = ACCEPTED / f"{stems[0]}.py"
        cleanup = None
    else:
        joined = "\n\n".join((ACCEPTED / f"{stem}.py").read_text() for stem in stems)
        source_path = Path(tempfile.mkdtemp()) / "group.py"
        source_path.write_text(joined)
        cleanup = source_path.parent

    try:
        finished = subprocess.run(
            ["cargo", "run", "-q", "-p", "compylr-cli", "--", "--emit", "ir", str(source_path)],
            capture_output=True,
            text=True,
            cwd=REPO,
            check=False,
        )
        if finished.returncode != 0:
            raise RuntimeError(f"emitting IR for {', '.join(stems)} failed:\n{finished.stderr}")
        parsed: dict[str, object] = json.loads(finished.stdout)
        return parsed
    finally:
        if cleanup is not None:
            shutil.rmtree(cleanup, ignore_errors=True)


def agreeing_fixtures() -> set[str]:
    """The fixtures whose translation agrees with CPython.

    Established by running the translation tier. It covers the corpus as one test, so either every
    accepted fixture agreed or the evidence for all of them is absent -- and in that case nothing
    is claimed, which is the honest answer rather than a table built on a failing suite.
    """
    finished = subprocess.run(
        [
            "cargo",
            "test",
            "-q",
            "-p",
            "compylr",
            "--test",
            "differential",
            "the_whole_accepted_corpus_agrees_with_cpython",
        ],
        capture_output=True,
        text=True,
        cwd=REPO,
        check=False,
    )
    if finished.returncode != 0:
        print(
            "the translation tier does not pass, so no construct can be reported as accepted:\n"
            f"{finished.stdout}\n{finished.stderr}",
            file=sys.stderr,
        )
        return set()
    return set(fixture_stems())


def matrix_body(tags_by_fixture: dict[str, set[str]], agreed: set[str]) -> str:
    """The table, built from coverage and evidence.

    Pure: everything that reads the compiler or the disk happens above this, so what the table
    says given a set of facts is testable without building anything.
    """
    tables = _tables()
    first_use: dict[str, str] = {}
    for stem in sorted(tags_by_fixture):
        if stem not in agreed:
            continue
        for tag in tags_by_fixture[stem]:
            first_use.setdefault(tag, stem)

    rows: list[str] = []
    covered = 0
    for table, forms in tables.items():
        kind = KINDS.get(table, table)
        for form in forms:
            stem = first_use.get(form)
            if stem is None:
                continue
            covered += 1
            rows.append(f"| `{form}` | {kind} | `{stem}.py` |")

    if not rows:
        return "_No construct can be reported as accepted: the translation tier does not pass._"

    total = sum(len(forms) for forms in tables.values())
    header = [
        f"{covered} of {total} IR forms are exercised by a fixture that translated, built, ran, "
        "and agreed with CPython. A form with no such fixture is not listed.",
        "",
        "| Form | Kind | Exercised by |",
        "| --- | --- | --- |",
    ]
    return "\n".join(header + rows)


def generate() -> str:
    """Gather the evidence and build the table."""
    agreed = agreeing_fixtures()
    tags_by_fixture: dict[str, set[str]] = {}
    for stems in groups():
        found = tags(emit_ir(stems))
        # A group's forms belong to every fixture in it: the unit is what has them, and which
        # member of a pair "owns" a cross-source call is not a question the IR answers.
        for stem in stems:
            tags_by_fixture[stem] = found
    return matrix_body(tags_by_fixture, agreed)


def compare(region: Region, body: str) -> str | None:
    """`None` when the published text already matches, otherwise what differs."""
    text = region.path.read_text()
    start, end = find_region(text, region)
    published = text[start:end].strip()
    if published == body.strip():
        return None
    return (
        f"{region.path.relative_to(REPO) if region.path.is_relative_to(REPO) else region.path}"
        f"  {region.name}: published text differs from what regeneration would produce\n"
        f"--- published ---\n{published}\n--- regenerated ---\n{body.strip()}"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the published matrix is what regeneration would produce; write nothing",
    )
    parser.add_argument(
        "--markers",
        action="store_true",
        help="verify only that the region is addressable; runs no compiler",
    )
    arguments = parser.parse_args(argv)

    try:
        if arguments.markers:
            # Presence only. Regenerating the matrix runs the compiler, and a commit hook is not
            # the place for that -- the same reason `update_benchmarks.py` checks markers rather
            # than re-measuring. Moving or renaming a marker is what breaks the generator, and
            # this is what catches it in the second it takes.
            find_region(MATRIX.path.read_text(), MATRIX)
            print(f"ok  {MATRIX.path.name}  {MATRIX.name}")
            return 0

        body = generate()
        if arguments.check:
            difference = compare(MATRIX, body)
            if difference is not None:
                print(difference, file=sys.stderr)
                print("run ./scripts/update_subset.py", file=sys.stderr)
                return 1
            print(f"ok  {MATRIX.path.name}  {MATRIX.name}")
            return 0

        text = MATRIX.path.read_text()
        MATRIX.path.write_text(replace_region(text, MATRIX, body))
        print(f"rewrote  {MATRIX.path.name}  {MATRIX.name}")
        return 0
    except (MarkerError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
