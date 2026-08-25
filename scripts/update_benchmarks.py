#!/usr/bin/env python3
"""Run the demo's benchmarks and write the results back into the READMEs.

    python scripts/update_benchmarks.py               # measure, then rewrite both READMEs
    python scripts/update_benchmarks.py --scale 4     # bigger inputs
    python scripts/update_benchmarks.py --dry-run     # measure and print, write nothing
    python scripts/update_benchmarks.py --check       # markers only; measures nothing

A README table of timings is the kind of thing that is true when it is written and quietly false
six months later, which is worse than absent: a reader trusts it. So the tables are **generated**
into marked regions and the regions are rewritten from a real run.

Three things this deliberately does *not* do.

It does not summarise. The demo's benchmark prints its own header, its own noise floor and its own
agreement line, and all of it goes into the README verbatim. A number without the floor it was
measured against is not a measurement, and dropping the header would lose the scale the run used.

It does not interpret. The prose around each block is written by hand and stays that way, because
what a row *means* -- that conversion is proportional to the size of the argument, that a warm
cache hit loses -- is not something a run can tell you.

It does not hide where it ran. Every block carries the machine, the interpreter and the date, so a
reader comparing two revisions can see whether they are comparable at all. CI runners are shared
hardware and their noise floor is much worse than a quiet laptop's; the floor is printed right
there in the block, which is the honest way to say so.
"""

from __future__ import annotations

import argparse
import platform
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEMO = REPO / "demo"
DEMO_TS = REPO / "demo-ts"

#: How a generated region is delimited. The name is what makes a region addressable, so a block
#: can be moved or reordered in the README without this script losing track of it.
OPEN = "<!-- benchmark:{name} -->"
CLOSE = "<!-- /benchmark:{name} -->"

#: One row of a printed table: label, compiled µs, interpreted µs, ratio.
ROW = re.compile(
    r"^(?P<label>\S.*?)\s{2,}(?P<fast>[\d.]+)us\s+(?P<slow>[\d.]+)us\s+(?P<ratio>\S+)$"
)


@dataclass(frozen=True)
class Region:
    """A generated block: which file it lives in, and what produces its contents."""

    name: str
    path: Path


ALGORITHMS = Region("algorithms", DEMO / "README.md")
NTH_PRIME = Region("nth-prime", DEMO / "README.md")
SUMMARY = Region("summary", REPO / "README.md")
TS_ALGORITHMS = Region("ts-algorithms", DEMO_TS / "README.md")
TS_NTH_PRIME = Region("ts-nth-prime", DEMO_TS / "README.md")

REGIONS = (ALGORITHMS, NTH_PRIME, SUMMARY, TS_ALGORITHMS, TS_NTH_PRIME)


class MarkerError(RuntimeError):
    """A region is missing, duplicated, or malformed.

    Raised rather than silently skipped: a rewrite that quietly writes nothing is how a table
    goes stale while the job that was supposed to keep it fresh reports success.
    """


def find_region(text: str, name: str) -> tuple[int, int]:
    """Return the character span *between* a region's markers.

    The markers themselves are left in place, so rewriting a region cannot lose it.
    """
    opening, closing = OPEN.format(name=name), CLOSE.format(name=name)
    if text.count(opening) != 1 or text.count(closing) != 1:
        raise MarkerError(
            f"expected exactly one {opening} and one {closing}; "
            f"found {text.count(opening)} and {text.count(closing)}"
        )
    start = text.index(opening) + len(opening)
    end = text.index(closing)
    if end < start:
        raise MarkerError(f"{closing} appears before {opening}")
    return start, end


def replace_region(text: str, name: str, body: str) -> str:
    """Return `text` with the named region's contents replaced by `body`."""
    start, end = find_region(text, name)
    return f"{text[:start]}\n{body.strip()}\n{text[end:]}"


def provenance(detail: str) -> str:
    """One line saying what was measured, where, and when.

    The machine matters more than it looks like it should. A shared CI runner and a quiet laptop
    disagree by more than most of the differences anyone wants to read out of these tables.
    """
    machine = f"{platform.system()} {platform.machine()}"
    interpreter = f"Python {platform.python_version()}"
    when = datetime.now(UTC).strftime("%Y-%m-%d")
    return f"_{detail} — measured on {machine}, {interpreter}, {when}._"


def ts_provenance(detail: str) -> str:
    """One line saying what was measured for TypeScript demo, where, and when."""
    machine = f"{platform.system()} {platform.machine()}"
    when = datetime.now(UTC).strftime("%Y-%m-%d")
    return f"_{detail} — measured on {machine}, Node.js 22, {when}._"


def run_benchmark(module: str, args: list[str]) -> str:
    """Run one of the demo's benchmarks and return its table.

    Run through `uv` from the demo directory, which is how the Makefile and the demo's own README
    run it -- the demo consumes compylr from this checkout, so measuring it any other way would
    risk timing an installed release instead of the working tree.
    """
    completed = subprocess.run(
        ["uv", "run", "python", "-m", module, *args],
        cwd=DEMO,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"{module} failed ({completed.returncode}):\n{completed.stdout}\n{completed.stderr}"
        )
    output = completed.stdout.strip()
    if "WARNING: compiled and interpreted disagreed" in output:
        # Every timing in the table would be comparing two different computations.
        raise SystemExit(f"the two modes disagreed, so the numbers mean nothing:\n{output}")
    return output


def run_ts_benchmark(script: str, args: list[str]) -> str:
    """Run one of the TypeScript demo's benchmarks and return its table."""
    completed = subprocess.run(
        ["node", "--experimental-strip-types", script, *args],
        cwd=DEMO_TS,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"{script} failed ({completed.returncode}):\n{completed.stdout}\n{completed.stderr}"
        )
    return completed.stdout.strip()


def parse_rows(table: str) -> list[tuple[str, float, float, str]]:
    """Every data row of a printed table, as (label, compiled µs, interpreted µs, ratio)."""
    rows = []
    for line in table.splitlines():
        match = ROW.match(line.rstrip())
        if match:
            rows.append(
                (
                    match["label"].strip(),
                    float(match["fast"]),
                    float(match["slow"]),
                    match["ratio"],
                )
            )
    if not rows:
        raise SystemExit(f"no rows could be read out of the table:\n{table}")
    return rows


def summarise(table: str, *, edges: int = 3) -> str:
    """The ends of the table, for the root README.

    The ends rather than a headline, because the spread is the finding: the top is arithmetic in a
    tight loop and the bottom is work dominated by crossing the boundary. A single number would be
    the one claim this project should not make.
    """
    rows = [row for row in parse_rows(table) if not row[0].startswith("reference")]
    control = next((row for row in parse_rows(table) if row[0].startswith("reference")), None)

    lines = ["| workload | compiled | interpreted | speedup |", "| --- | ---: | ---: | ---: |"]
    for label, fast, slow, ratio in rows[:edges]:
        lines.append(f"| `{label}` | {fast:.2f}us | {slow:.2f}us | **{ratio}** |")
    if len(rows) > 2 * edges:
        lines.append("| … | | | |")
    for label, fast, slow, ratio in rows[-edges:]:
        lines.append(f"| `{label}` | {fast:.2f}us | {slow:.2f}us | **{ratio}** |")
    if control is not None:
        lines.append(f"| `{control[0]}` | {control[1]:.2f}us | {control[2]:.2f}us | {control[3]} |")
    return "\n".join(lines)


def fenced(table: str) -> str:
    """A table as a fenced block, verbatim."""
    return f"```\n{table}\n```"


def check_markers() -> int:
    """Verify every region is addressable. Measures nothing."""
    for region in REGIONS:
        text = region.path.read_text()
        find_region(text, region.name)
        print(f"ok  {region.path.relative_to(REPO)}  {region.name}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="scripts/update_benchmarks.py",
        description="Run the demo's benchmarks and write the results into the READMEs.",
    )
    parser.add_argument("--scale", type=int, default=1, help="input size for the algorithm table")
    parser.add_argument("--n", type=int, default=500, help="which prime the nth-prime table times")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the README regions are addressable and exit; measures nothing",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="measure and print, but write nothing"
    )
    args = parser.parse_args(argv)

    if args.check:
        try:
            return check_markers()
        except MarkerError as error:
            print(f"error: {error}", file=sys.stderr)
            return 1

    # Fail on a missing marker before spending minutes measuring.
    check_markers()

    algorithms = run_benchmark("algorithms.benchmark", ["--scale", str(args.scale)])
    nth_prime = run_benchmark("algorithms.nth_prime.benchmark", ["--n", str(args.n)])
    ts_algorithms = run_ts_benchmark("src/algorithms/benchmark.ts", ["--scale", str(args.scale)])
    ts_nth_prime = run_ts_benchmark("src/algorithms/nth_prime/benchmark.ts", ["--n", str(args.n)])

    bodies = {
        ALGORITHMS: f"{fenced(algorithms)}\n\n{provenance(f'scale {args.scale}')}",
        NTH_PRIME: f"{fenced(nth_prime)}\n\n{provenance(f'n = {args.n}')}",
        SUMMARY: f"{summarise(algorithms)}\n\n{provenance(f'scale {args.scale}')}",
        TS_ALGORITHMS: f"{fenced(ts_algorithms)}\n\n{ts_provenance(f'scale {args.scale}')}",
        TS_NTH_PRIME: f"{fenced(ts_nth_prime)}\n\n{ts_provenance(f'n = {args.n}')}",
    }

    if args.dry_run:
        for region, body in bodies.items():
            print(f"--- {region.path.relative_to(REPO)} :: {region.name}\n{body}\n")
        return 0

    for path in {region.path for region in REGIONS}:
        text = path.read_text()
        for region, body in bodies.items():
            if region.path == path:
                text = replace_region(text, region.name, body)
        path.write_text(text)
        print(f"wrote {path.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
