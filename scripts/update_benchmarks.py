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
from datetime import UTC, datetime
from pathlib import Path

from _regions import MarkerError, Region, find_region, replace_region

REPO = Path(__file__).resolve().parents[1]
DEMOS_DIR = REPO / "demo"

#: How a generated region is delimited. The name is what makes a region addressable, so a block
#: can be moved or reordered in the README without this script losing track of it.

#: One row of a printed table: label, compiled µs, interpreted µs, ratio.
ROW = re.compile(
    r"^(?P<label>\S.*?)\s{2,}(?P<fast>[\d.]+)us\s+(?P<slow>[\d.]+)us\s+(?:(?P<spread>[\d]+%|--)\s+)?(?P<ratio>.+)$"
)

SUMMARY = Region("summary", REPO / "README.md")


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


def run_benchmark(cwd: Path, module: str, args: list[str]) -> str:
    """Run one of the demo's benchmarks and return its table.

    Run through `uv` from the demo directory, which is how the Makefile and the demo's own README
    run it -- the demo consumes compylr from this checkout, so measuring it any other way would
    risk timing an installed release instead of the working tree.
    """
    completed = subprocess.run(
        ["uv", "run", "python", "-m", module, *args],
        cwd=cwd,
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


def run_ts_benchmark(cwd: Path, script: str, args: list[str]) -> str:
    """Run one of the TypeScript demo's benchmarks and return its table."""
    completed = subprocess.run(
        ["node", "--experimental-strip-types", script, *args],
        cwd=cwd,
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


def check_markers(regions: list[Region]) -> int:
    """Verify every region is addressable. Measures nothing."""
    for region in regions:
        text = region.path.read_text()
        find_region(text, region)
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

    demo_dirs = sorted(d for d in DEMOS_DIR.iterdir() if d.is_dir() and not d.name.startswith("."))
    regions = [SUMMARY]
    for demo_dir in demo_dirs:
        regions.append(Region("algorithms", demo_dir / "README.md"))
        regions.append(Region("nth-prime", demo_dir / "README.md"))

    if args.check:
        try:
            return check_markers(regions)
        except MarkerError as error:
            print(f"error: {error}", file=sys.stderr)
            return 1

    # Fail on a missing marker before spending minutes measuring.
    check_markers(regions)

    bodies = {}
    for demo_dir in demo_dirs:
        if (demo_dir / "pyproject.toml").exists():
            algorithms = run_benchmark(
                demo_dir, "algorithms.benchmark", ["--scale", str(args.scale)]
            )
            nth_prime = run_benchmark(
                demo_dir, "algorithms.nth_prime.benchmark", ["--n", str(args.n)]
            )
            prov = provenance(f"scale {args.scale}")
            prov_n = provenance(f"n = {args.n}")

            if demo_dir.name == "demo-python-rust":
                bodies[SUMMARY] = f"{summarise(algorithms)}\n\n{prov}"
        elif (demo_dir / "package.json").exists():
            algorithms = run_ts_benchmark(
                demo_dir, "src/algorithms/benchmark.ts", ["--scale", str(args.scale)]
            )
            nth_prime = run_ts_benchmark(
                demo_dir, "src/algorithms/nth_prime/benchmark.ts", ["--n", str(args.n)]
            )
            prov = ts_provenance(f"scale {args.scale}")
            prov_n = ts_provenance(f"n = {args.n}")
        else:
            print(f"warning: unknown demo type for {demo_dir.name}", file=sys.stderr)
            continue

        bodies[Region("algorithms", demo_dir / "README.md")] = f"{fenced(algorithms)}\n\n{prov}"
        bodies[Region("nth-prime", demo_dir / "README.md")] = f"{fenced(nth_prime)}\n\n{prov_n}"

    if args.dry_run:
        for region, body in bodies.items():
            print(f"--- {region.path.relative_to(REPO)} :: {region.name}\n{body}\n")
        return 0

    for path in {region.path for region in regions}:
        text = path.read_text()
        for region, body in bodies.items():
            if region.path == path and region in bodies:
                text = replace_region(text, region, body)
        path.write_text(text)
        print(f"wrote {path.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
