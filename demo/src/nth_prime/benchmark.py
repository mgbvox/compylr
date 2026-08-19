"""Time each variant compiled against the same code interpreted.

**Run in two processes, deliberately.** Measuring the interpreted side in the same process as the
compiled one would be a lie: `recursive_nth_prime`'s body calls `recursive_next_prime` through a
module global, and that global is the *compiled* wrapper. Reaching for the original through
`CompiledFunction.python_function` gets you an interpreted outer function calling compiled inner
ones — a number that means nothing.

`COMPYLR_DISABLE=1` is what makes an honest measurement possible: in that process every marked
member is left exactly as written, so an interpreted call stays interpreted all the way down.

    python -m nth_prime.benchmark            # compare both modes
    python -m nth_prime.benchmark --n 500    # a bigger workload

Timings are the **best** of several repetitions rather than the mean. Noise only ever adds, so the
minimum is the closest estimate of the work itself; a mean mostly measures the machine's mood.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from collections.abc import Callable
from dataclasses import dataclass
from typing import TypedDict

#: How many times each workload is timed. The best is kept.
REPETITIONS = 5

#: Environment variable that turns compilation off. Stated here rather than imported, so this
#: module can name it before deciding whether to import compylr at all.
DISABLE_ENV = "COMPYLR_DISABLE"


class Measurement(TypedDict):
    """What one process reports about its own mode.

    A typed shape rather than a loose dict: the two halves of the comparison are produced by
    separate processes and joined through JSON, which is exactly where a renamed key would go
    unnoticed until the table printed nonsense.
    """

    compiled: bool
    n: int
    repetitions: int
    seconds: dict[str, float]
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


#: A timed batch must last at least this long to mean anything.
#:
#: `perf_counter` has a resolution in the tens of nanoseconds, so a call that takes hundreds of
#: nanoseconds — a warm cache hit does — measures as either zero or one tick. Timing a batch and
#: dividing is the difference between a number and a coin flip.
MINIMUM_BATCH_SECONDS = 0.01


def _fresh_call(build: Callable[[], Callable[[int], int]], n: int) -> Callable[[], int]:
    """A zero-argument callable that builds a fresh implementation and runs it once.

    A function rather than a lambda in the loop: a lambda would capture `build` by name and every
    iteration would end up timing the last workload. It happens to work when the closure is called
    immediately, which is exactly why it is the kind of bug that survives a long time.
    """
    return lambda: build()(n)


def _per_call(call: Callable[[], object], repetitions: int) -> float:
    """Seconds per call, from the fastest of `repetitions` batches.

    The batch size is calibrated upward until one batch is long enough to time, so a fast workload
    and a slow one are both measured honestly rather than one of them reading as zero.
    """
    batch = 1
    while True:
        started = time.perf_counter()
        for _ in range(batch):
            call()
        elapsed = time.perf_counter() - started
        if elapsed >= MINIMUM_BATCH_SECONDS or batch >= 1_000_000:
            break
        # Scale by how far short it fell, with a floor so this converges quickly.
        batch *= max(2, int(MINIMUM_BATCH_SECONDS / elapsed) + 1) if elapsed > 0 else 10

    best = elapsed / batch
    for _ in range(repetitions - 1):
        started = time.perf_counter()
        for _ in range(batch):
            call()
        best = min(best, (time.perf_counter() - started) / batch)
    return best


def measure(n: int, repetitions: int = REPETITIONS) -> Measurement:
    """Time every workload in *this* process's mode."""
    from . import memoized

    seconds: dict[str, float] = {}
    answers: dict[str, int] = {}

    for workload in workloads():
        build = workload.build
        # Called once first so the first call's module resolution is not timed with the work.
        answers[workload.key] = build()(n)
        seconds[workload.key] = _per_call(_fresh_call(build, n), repetitions)

    # The warm cache, which is the entire point of memoizing: a second request for an `n` this
    # instance has already answered.
    warm = memoized.PrimeCache()
    warm.nth(n)
    seconds["memoized_warm"] = _per_call(lambda: warm.nth(n), repetitions)
    answers["memoized_warm"] = warm.nth(n)

    return Measurement(
        compiled=_compylr_enabled(),
        n=n,
        repetitions=repetitions,
        seconds=seconds,
        answers=answers,
    )


def _compylr_enabled() -> bool:
    """Whether this process actually compiled, asked of the manager rather than of the environment.

    The environment is what the caller *requested*; the manager is what happened. A project calling
    `initialize(enabled=False)` itself would otherwise be reported as compiled.
    """
    from ._compylr import c

    return c.enabled


def _run_child(n: int, repetitions: int, *, disabled: bool) -> Measurement:
    """Measure in a fresh process, with compilation on or off."""
    env = dict(os.environ)
    if disabled:
        env[DISABLE_ENV] = "1"
    else:
        env.pop(DISABLE_ENV, None)

    result = subprocess.run(
        [sys.executable, "-m", "nth_prime.benchmark",
         "--n", str(n), "--repetitions", str(repetitions), "--emit-json"],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        mode = "interpreted" if disabled else "compiled"
        raise RuntimeError(f"the {mode} run failed:\n{result.stderr}")
    measured: Measurement = json.loads(result.stdout)
    return measured


#: Every row of the comparison, in the order it is printed.
ROWS = [
    ("reference", "reference (never compiled)"),
    ("recursive", "recursive"),
    ("iterative", "iterative"),
    ("memoized", "memoized (cold cache)"),
    ("memoized_warm", "memoized (warm cache)"),
]


def format_comparison(compiled: Measurement, interpreted: Measurement) -> str:
    """The comparison, as a table."""
    fast = compiled["seconds"]
    slow = interpreted["seconds"]

    lines = [
        f"nth prime, n={compiled['n']}, per call, best of {compiled['repetitions']} batches",
        "",
        f"{'variant':<28}{'compiled':>13}{'interpreted':>15}{'speedup':>10}",
        f"{'-' * 28}{'-' * 13}{'-' * 15}{'-' * 10}",
    ]
    for key, label in ROWS:
        a, b = fast[key] * 1e6, slow[key] * 1e6
        ratio = f"{b / a:.1f}x" if a > 0 else "n/a"
        lines.append(f"{label:<28}{a:>11.2f}us{b:>13.2f}us{ratio:>10}")

    lines.append("")
    floor = slow["reference"] / fast["reference"] if fast["reference"] else 0.0
    lines.append(
        f"The reference is never compiled, so its {floor:.2f}x is this run's noise floor — "
        "read every other row against that, not against 1.0."
    )

    # A disagreement would make every timing above meaningless.
    mismatched = [
        key for key, _ in ROWS if compiled["answers"][key] != interpreted["answers"][key]
    ]
    if mismatched:
        lines.append(f"WARNING: compiled and interpreted disagreed for {mismatched}")
    else:
        lines.append("Both modes returned the same answer for every variant.")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m nth_prime.benchmark",
        description="Time each variant compiled against the same code interpreted.",
        epilog=(
            "The two modes run in separate processes. Measuring both in one would be dishonest: a "
            "marked function calls other marked functions through module globals, so an "
            f"'interpreted' outer call would still reach compiled inner ones. {DISABLE_ENV}=1 is "
            "what makes an interpreted run interpreted all the way down."
        ),
    )
    parser.add_argument("--n", type=int, default=200, help="which prime to compute")
    parser.add_argument("--repetitions", type=int, default=REPETITIONS,
                        help="how many times to time each workload; the best is kept")
    parser.add_argument("--emit-json", action="store_true",
                        help="measure this process only and print JSON (used by the driver)")
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
