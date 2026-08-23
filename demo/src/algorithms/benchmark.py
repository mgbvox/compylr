"""Time every algorithm compiled against the same code interpreted.

    python -m algorithms.benchmark              # the whole table
    python -m algorithms.benchmark --scale 4    # bigger inputs

The method is `_timing`'s and is shared with `nth_prime.benchmark`: two processes, batches rather
than single calls, the best batch rather than the mean, and every batch kept so a figure can be
read with its uncertainty. Read that module before trusting a number from this one.

**Read the speedup column against the noise floor, not against 1.0.** The floor comes from the
never-compiled `reference` row, whose true ratio is exactly 1.0 by construction, so whatever it
reports instead is this machine's noise. A row that does not clear the floor prints "not
resolvable" rather than a ratio — several workloads here move by more between identical runs than
the improvements anyone would want to measure, and a bare number would invite reading one as the
other.

What this table is for is the **spread**. A demo that reported one speedup would be hiding the
thing worth knowing: compiling is not uniformly good. Arithmetic in a tight loop wins by a lot,
because there is nothing for the interpreter to do but dispatch. Work that is mostly moving a
large collection across the boundary wins by much less, because the conversion is real and the
interpreted version was already calling into C. And `joined` **loses**, because it is quadratic
string concatenation either way and Python's `str` is very good at it.

`reference` is the control: it is never compiled, so its ratio is what "no difference" looks like
on the machine you ran this on. Read every other row against that rather than against 1.0.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Callable
from dataclasses import dataclass
from random import Random
from typing import Any, TypedDict

from ._timing import (
    DISABLE_ENV,
    NOT_RESOLVABLE,
    REPETITIONS,
    UNSTABLE_MARK,
    UNSTABLE_SPREAD,
    Timing,
    compylr_enabled,
    format_ratio,
    measure_call,
    measure_in_child,
    noise_floor,
    uncertainty,
    unstable,
)

#: Fixed, so the two processes measure the same work and two runs are comparable.
SEED = 20260821


class Measurement(TypedDict):
    """What one process reports about its own mode."""

    compiled: bool
    scale: int
    repetitions: int
    #: Every batch's seconds-per-call, not just the best. The spread between them is the only
    #: evidence a reader has about whether the best means anything.
    samples: dict[str, list[float]]
    answers: dict[str, str]


@dataclass(frozen=True)
class Workload:
    """One thing to time, and the signature of what it returned."""

    key: str
    label: str
    call: Callable[[], Any]


def _signature(value: Any) -> str:
    """A stable string for an answer, so two processes can be compared.

    Mappings and sets are rendered sorted. Their iteration order is not guaranteed and varies
    between runs, so `repr` of one is not a fact about the computation — comparing it directly
    would report a disagreement that is really the hash seed.
    """
    if isinstance(value, dict):
        return "{" + ", ".join(f"{k!r}: {_signature(v)}" for k, v in sorted(value.items())) + "}"
    if isinstance(value, set | frozenset):
        return "{" + ", ".join(sorted(repr(v) for v in value)) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(_signature(v) for v in value) + "]"
    if isinstance(value, float):
        # Rounded, because the last bit of a float is not something two processes have to agree
        # on to be computing the same thing.
        return f"{value:.9g}"
    return repr(value)


def workloads(scale: int) -> list[Workload]:
    """Every timed workload, sized by `scale`.

    Built fresh in each process rather than shared, so the two modes get identical inputs without
    either of them having to serialise a list of ten thousand integers through JSON.
    """
    from . import arithmetic, dynamic, graphs, matrices, sorting, stats, structures, text
    from .nth_prime import reference

    source = Random(SEED)
    numbers = [source.randint(-10_000, 10_000) for _ in range(200 * scale)]
    reals = [source.uniform(-1_000.0, 1_000.0) for _ in range(500 * scale)]
    vocabulary = [f"w{n}" for n in range(50)]
    words = [source.choice(vocabulary) for _ in range(500 * scale)]
    tokens_left = [source.choice("abcdef") for _ in range(30 * scale)]
    tokens_right = [source.choice("abcdef") for _ in range(30 * scale)]
    side = 12 * scale
    square = [[source.randint(-9, 9) for _ in range(side)] for _ in range(side)]
    nodes = 200 * scale
    graph = {node: [(node * 7 + step) % nodes for step in range(1, 4)] for node in range(nodes)}
    edges = [(source.randrange(nodes), source.randrange(nodes)) for _ in range(nodes)]
    ordered = sorted(numbers)

    return [
        # The control. Never compiled, so its ratio is this run's noise floor.
        Workload("reference", "reference (never compiled)", lambda: reference.nth_prime(60)),
        Workload("merge_sort", "sorting.merge_sort", lambda: sorting.merge_sort(numbers)),
        Workload(
            "insertion_sort", "sorting.insertion_sort", lambda: sorting.insertion_sort(ordered)
        ),
        Workload("sieve", "arithmetic.sieve", lambda: arithmetic.sieve(200 * scale)),
        Workload(
            "collatz", "arithmetic.collatz_length", lambda: arithmetic.collatz_length(97 * scale)
        ),
        Workload("deviation", "stats.standard_deviation", lambda: stats.standard_deviation(reals)),
        Workload("normalize", "stats.normalize", lambda: stats.normalize(reals)),
        Workload("word_count", "text.word_count", lambda: text.word_count(words)),
        # A single length read per element, so what it mostly measures is what iterating a
        # collection of owned values costs. That makes it the row where borrowing the loop
        # variable rather than copying it is visible.
        Workload("total_length", "text.total_length", lambda: text.total_length(words)),
        Workload("joined", "text.joined", lambda: text.joined(words, "-")),
        Workload("bfs", "graphs.bfs_distances", lambda: graphs.bfs_distances(graph, 0)),
        Workload(
            "topological", "graphs.topological_order", lambda: graphs.topological_order(graph)
        ),
        Workload(
            "edit_distance",
            "dynamic.edit_distance",
            lambda: dynamic.edit_distance(tokens_left, tokens_right),
        ),
        Workload(
            "knapsack",
            "dynamic.knapsack",
            lambda: dynamic.knapsack(
                [abs(n) % 20 + 1 for n in numbers[:40]],
                [abs(n) % 50 + 1 for n in numbers[:40]],
                60 * scale,
            ),
        ),
        Workload("multiply", "matrices.multiply", lambda: matrices.multiply(square, square)),
        Workload("transpose", "matrices.transpose", lambda: matrices.transpose(square)),
        Workload(
            "components",
            "structures.component_count",
            lambda: structures.component_count(nodes, edges),
        ),
    ]


def measure(scale: int, repetitions: int = REPETITIONS) -> Measurement:
    """Time every workload in *this* process's mode."""
    samples: dict[str, list[float]] = {}
    answers: dict[str, str] = {}
    for workload in workloads(scale):
        # Called once first, so the first call's module resolution is not timed with the work.
        answers[workload.key] = _signature(workload.call())
        samples[workload.key] = list(measure_call(workload.call, repetitions).samples)
    return Measurement(
        compiled=compylr_enabled(),
        scale=scale,
        repetitions=repetitions,
        samples=samples,
        answers=answers,
    )


def _run_child(scale: int, repetitions: int, *, disabled: bool) -> Measurement:
    """Measure in a fresh process, with compilation on or off."""
    measured: Measurement = measure_in_child(  # type: ignore[assignment]
        "algorithms.benchmark",
        ["--scale", str(scale), "--repetitions", str(repetitions)],
        disabled=disabled,
    )
    return measured


def _timings(measured: Measurement) -> dict[str, Timing]:
    """Every workload's batches, as timings that know their own spread."""
    return {key: Timing(tuple(batches)) for key, batches in measured["samples"].items()}


def format_comparison(compiled: Measurement, interpreted: Measurement) -> str:
    """The comparison, as a table sorted by how much compiling helped.

    Every ratio is read against a floor rather than against 1.0, and a ratio that does not clear
    its floor is named rather than printed. A table of bare numbers invites exactly the mistake
    this benchmark exists to prevent: reading a 1.1x off a harness that moves by 30%.
    """
    fast, slow = _timings(compiled), _timings(interpreted)
    labels = {workload.key: workload.label for workload in workloads(compiled["scale"])}
    floor = noise_floor(fast["reference"], slow["reference"])

    ranked = sorted(
        fast, key=lambda key: -(slow[key].best / fast[key].best) if fast[key].best else 0.0
    )
    lines = [
        (
            f"every algorithm, scale={compiled['scale']}, per call, "
            f"best of {compiled['repetitions']} batches"
        ),
        "",
        f"{'workload':<32}{'compiled':>13}{'interpreted':>15}{'spread':>9}{'speedup':>17}",
        f"{'-' * 32}{'-' * 13}{'-' * 15}{'-' * 9}{'-' * 17}",
    ]
    shaky = []
    for key in ranked:
        quick, slowly = fast[key], slow[key]
        mark = UNSTABLE_MARK if unstable(quick, slowly) else ""
        if mark:
            shaky.append(labels[key])
        # The wider of the two modes: it is what limits what this row can support.
        widest = max(quick.spread, slowly.spread)
        ratio = (
            format_ratio(slowly.best / quick.best, uncertainty(floor, quick, slowly))
            if quick.best > 0
            else "n/a"
        )
        lines.append(
            f"{labels[key] + mark:<32}{quick.best * 1e6:>11.2f}us"
            f"{slowly.best * 1e6:>13.2f}us{widest:>8.0%}{ratio:>17}"
        )

    lines += [
        "",
        (
            f"The reference is never compiled, so its true ratio is exactly 1.0 and everything it "
            f"reports instead is this run's noise floor: {floor:.0%}. A row closer to 1.0 "
            f'than that reads "{NOT_RESOLVABLE}" rather than a figure, because it would be one.'
        ),
        (
            "`spread` is how far the slowest batch ran from the fastest, in the mode that varied "
            "more. A row must clear its own spread as well as the floor to report a figure."
        ),
    ]
    if shaky:
        lines.append(
            f"{UNSTABLE_MARK} marks a workload whose batches varied by more than "
            f"{UNSTABLE_SPREAD:.0%}: unstable enough that its own figure is not worth reading. "
            f"({', '.join(shaky)})"
        )

    disagreed = [key for key in fast if compiled["answers"][key] != interpreted["answers"][key]]
    if disagreed:
        # Every timing above would be measuring two different computations.
        lines.append(f"WARNING: compiled and interpreted disagreed for {disagreed}")
    else:
        lines.append("Both modes returned the same answer for every workload.")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m algorithms.benchmark",
        description="Time every algorithm compiled against the same code interpreted.",
        epilog=(
            "The two modes run in separate processes. Measuring both in one would be dishonest: a "
            "marked function calls other marked functions through module globals, so an "
            f"'interpreted' outer call would still reach compiled inner ones. {DISABLE_ENV}=1 is "
            "what makes an interpreted run interpreted all the way down."
        ),
    )
    parser.add_argument("--scale", type=int, default=1, help="how big the inputs are")
    parser.add_argument(
        "--repetitions",
        type=int,
        default=REPETITIONS,
        help="how many times to time each workload; the best is kept",
    )
    parser.add_argument(
        "--emit-json",
        action="store_true",
        help="measure this process only and print JSON (used by the driver)",
    )
    args = parser.parse_args(argv)

    if args.emit_json:
        print(json.dumps(measure(args.scale, args.repetitions)))
        return 0

    compiled = _run_child(args.scale, args.repetitions, disabled=False)
    interpreted = _run_child(args.scale, args.repetitions, disabled=True)

    if not compiled["compiled"]:
        print(
            f"the compiled run reported that compylr was disabled; is {DISABLE_ENV} set outside "
            "this benchmark, or does the project call initialize(enabled=False)?",
            file=sys.stderr,
        )
        return 1
    if interpreted["compiled"]:  # pragma: no cover - would mean the switch did nothing
        print("the interpreted run reported that it compiled anyway", file=sys.stderr)
        return 1

    print(format_comparison(compiled, interpreted))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
