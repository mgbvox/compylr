"""Time each variant compiled against the same code interpreted.

**Run in two processes, deliberately.** Measuring the interpreted side in the same process as the
compiled one would be a lie: `recursive_nth_prime`'s body calls `recursive_next_prime` through a
module global, and that global is the *compiled* wrapper. Reaching for the original through
`CompiledFunction.python_function` gets you an interpreted outer function calling compiled inner
ones — a number that means nothing.

`COMPYLR_DISABLE=1` is what makes an honest measurement possible: in that process every marked
member is left exactly as written, so an interpreted call stays interpreted all the way down.

    python -m algorithms.nth_prime.benchmark            # compare both modes
    python -m algorithms.nth_prime.benchmark --n 500    # a bigger workload

Timings are the **best** of several repetitions rather than the mean. Noise only ever adds, so the
minimum is the closest estimate of the work itself; a mean mostly measures the machine's mood.

Every batch is kept, though, not only the best, because the spread between them is the only
evidence a reader has that the best means anything. A ratio is read against the noise floor the
never-compiled reference establishes, and one that does not clear it prints "not resolvable"
instead of a figure.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Callable
from dataclasses import dataclass
from typing import TypedDict

from .._timing import (
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


class Measurement(TypedDict):
    """What one process reports about its own mode.

    A typed shape rather than a loose dict: the two halves of the comparison are produced by
    separate processes and joined through JSON, which is exactly where a renamed key would go
    unnoticed until the table printed nonsense.
    """

    compiled: bool
    n: int
    repetitions: int
    #: Every batch's seconds-per-call, not just the best. The spread between them is the only
    #: evidence a reader has about whether the best means anything.
    samples: dict[str, list[float]]
    answers: dict[str, int]


@dataclass(frozen=True)
class Workload:
    """One thing to time, and how to build a fresh callable for it."""

    key: str
    label: str
    build: Callable[[], Callable[[int], int]]


def workloads() -> list[Workload]:
    """Every variant, plus the interpreted reference as a control.

    The reference is included in both modes on purpose. It is never compiled, so its two numbers
    should match — and if they do not, the comparison is measuring the machine rather than compylr.
    """
    from . import iterative, memoized, recursive, reference

    return [
        Workload("reference", "reference (never compiled)", lambda: reference.nth_prime),
        Workload("recursive", "recursive", lambda: recursive.nth_prime),
        Workload("iterative", "iterative", lambda: iterative.nth_prime),
        # A fresh cache each repetition, so this measures the same work the others do rather than
        # the cache it filled on the first pass. The warm case is reported separately.
        Workload("memoized", "memoized (cold cache)", lambda: memoized.PrimeCache().nth),
    ]


def _fresh_call(build: Callable[[], Callable[[int], int]], n: int) -> Callable[[], int]:
    """A zero-argument callable that builds a fresh implementation and runs it once.

    A function rather than a lambda in the loop: a lambda would capture `build` by name and every
    iteration would end up timing the last workload. It happens to work when the closure is called
    immediately, which is exactly why it is the kind of bug that survives a long time.
    """
    return lambda: build()(n)


def measure(n: int, repetitions: int = REPETITIONS) -> Measurement:
    """Time every workload in *this* process's mode."""
    from . import memoized

    samples: dict[str, list[float]] = {}
    answers: dict[str, int] = {}

    for workload in workloads():
        build = workload.build
        # Called once first so the first call's module resolution is not timed with the work.
        answers[workload.key] = build()(n)
        samples[workload.key] = list(measure_call(_fresh_call(build, n), repetitions).samples)

    # The warm cache, which is the entire point of memoizing: a second request for an `n` this
    # instance has already answered.
    warm = memoized.PrimeCache()
    warm.nth(n)
    samples["memoized_warm"] = list(measure_call(lambda: warm.nth(n), repetitions).samples)
    answers["memoized_warm"] = warm.nth(n)

    return Measurement(
        compiled=compylr_enabled(),
        n=n,
        repetitions=repetitions,
        samples=samples,
        answers=answers,
    )


def _run_child(n: int, repetitions: int, *, disabled: bool) -> Measurement:
    """Measure in a fresh process, with compilation on or off."""
    measured: Measurement = measure_in_child(  # type: ignore[assignment]
        "algorithms.nth_prime.benchmark",
        ["--n", str(n), "--repetitions", str(repetitions)],
        disabled=disabled,
    )
    return measured


#: Every row of the comparison, in the order it is printed.
ROWS = [
    ("reference", "reference (never compiled)"),
    ("recursive", "recursive"),
    ("iterative", "iterative"),
    ("memoized", "memoized (cold cache)"),
    ("memoized_warm", "memoized (warm cache)"),
]


def _timings(measured: Measurement) -> dict[str, Timing]:
    """Every variant's batches, as timings that know their own spread."""
    return {key: Timing(tuple(batches)) for key, batches in measured["samples"].items()}


def format_comparison(compiled: Measurement, interpreted: Measurement) -> str:
    """The comparison, as a table.

    Read the same way as the breadth benchmark's, and by the same code: a ratio is compared
    against a floor rather than against 1.0, and one that does not clear its floor is named
    instead of printed.
    """
    fast, slow = _timings(compiled), _timings(interpreted)
    floor = noise_floor(fast["reference"], slow["reference"])

    lines = [
        f"nth prime, n={compiled['n']}, per call, best of {compiled['repetitions']} batches",
        "",
        f"{'variant':<30}{'compiled':>13}{'interpreted':>15}{'spread':>9}{'speedup':>17}",
        f"{'-' * 30}{'-' * 13}{'-' * 15}{'-' * 9}{'-' * 17}",
    ]
    shaky = []
    for key, label in ROWS:
        quick, slowly = fast[key], slow[key]
        mark = UNSTABLE_MARK if unstable(quick, slowly) else ""
        if mark:
            shaky.append(label)
        widest = max(quick.spread, slowly.spread)
        ratio = (
            format_ratio(slowly.best / quick.best, uncertainty(floor, quick, slowly))
            if quick.best > 0
            else "n/a"
        )
        lines.append(
            f"{label + mark:<30}{quick.best * 1e6:>11.2f}us"
            f"{slowly.best * 1e6:>13.2f}us{widest:>8.0%}{ratio:>17}"
        )

    lines.append("")
    lines.append(
        f"The reference is never compiled, so its true ratio is exactly 1.0 and everything it "
        f"reports instead is this run's noise floor: {floor:.0%}. A row closer to 1.0 than "
        f'that reads "{NOT_RESOLVABLE}" rather than a figure, because it would be one.'
    )
    if shaky:
        lines.append(
            f"{UNSTABLE_MARK} marks a variant whose batches varied by more than "
            f"{UNSTABLE_SPREAD:.0%}: unstable enough that its own figure is not worth reading. "
            f"({', '.join(shaky)})"
        )

    # A disagreement would make every timing above meaningless.
    mismatched = [key for key, _ in ROWS if compiled["answers"][key] != interpreted["answers"][key]]
    if mismatched:
        lines.append(f"WARNING: compiled and interpreted disagreed for {mismatched}")
    else:
        lines.append("Both modes returned the same answer for every variant.")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m algorithms.nth_prime.benchmark",
        description="Time each variant compiled against the same code interpreted.",
        epilog=(
            "The two modes run in separate processes. Measuring both in one would be dishonest: a "
            "marked function calls other marked functions through module globals, so an "
            f"'interpreted' outer call would still reach compiled inner ones. {DISABLE_ENV}=1 is "
            "what makes an interpreted run interpreted all the way down."
        ),
    )
    parser.add_argument("--n", type=int, default=200, help="which prime to compute")
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
        print(json.dumps(measure(args.n, args.repetitions)))
        return 0

    compiled = _run_child(args.n, args.repetitions, disabled=False)
    interpreted = _run_child(args.n, args.repetitions, disabled=True)

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
