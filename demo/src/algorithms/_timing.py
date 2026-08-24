"""Timing, and the two-process arrangement that makes a compiled-against-interpreted number honest.

Shared by both benchmarks in this project, deliberately. They measure different things — one
problem three ways, and forty algorithms once each — but the *method* has to be identical, or the
two sets of numbers cannot be read side by side. Two copies of it would drift.

Four decisions are load-bearing, and each is a way of being wrong that looks like a result:

**The two modes run in separate processes.** A marked function reaches other marked functions
through module globals, so an "interpreted" outer call in a compiled process would still land in
compiled code. `COMPYLR_DISABLE=1` is what makes an interpreted run interpreted all the way down.

**A batch is timed, not a call.** `perf_counter` resolves to tens of nanoseconds, and plenty of
what is measured here takes hundreds. Timing one call would report zero or one tick.

**The best of several batches is kept, not the mean.** Noise only ever adds, so the minimum is the
closest estimate of the work itself. A mean mostly measures how busy the machine was.

**Every batch is kept, not only the best.** This is the newest of the four and the one that makes
the other three readable. A single best-of figure hides instability rather than removing it:
`sorting.merge_sort` has been observed at 160, 202, 235, 256, 264 and 277us across runs of
binaries that were in some cases *byte-identical*. A reader given only the 160 cannot tell a real
10% gain from the harness moving, and several improvements worth making are worth 10-25%. So a
timing carries its `spread`, a run states the `noise_floor` derived from its never-compiled
control row, and a ratio that does not clear that floor is reported as `NOT_RESOLVABLE` rather
than printed as though it were a result.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

__all__ = [
    "DISABLE_ENV",
    "MINIMUM_BATCH_SECONDS",
    "NOT_RESOLVABLE",
    "REPETITIONS",
    "UNSTABLE_MARK",
    "UNSTABLE_SPREAD",
    "Timing",
    "compylr_enabled",
    "format_ratio",
    "measure_call",
    "measure_in_child",
    "noise_floor",
    "per_call",
    "resolves",
    "uncertainty",
    "unstable",
]

#: How many times each workload is timed. The best is kept, and the spread of all of them reported.
REPETITIONS = 5

#: Environment variable that turns compilation off for a whole process.
#:
#: Named here rather than imported from compylr, so this module can talk about the switch before
#: deciding whether to import the package it belongs to.
DISABLE_ENV = "COMPYLR_DISABLE"

#: A timed batch must last at least this long to mean anything.
MINIMUM_BATCH_SECONDS = 0.01

#: What a ratio is called when it does not clear the run's noise floor.
#:
#: Deliberately not a number. Printing "1.1x" for a difference the harness cannot resolve is the
#: specific dishonesty this module exists to prevent, and a reader skims numbers.
NOT_RESOLVABLE = "not resolvable"

#: How far a workload may swing run-to-run before its own figure is not worth reading.
#:
#: A quarter of the best batch. Chosen against the observed merge_sort range (73%), which must be
#: caught, and against the few percent an idle machine produces, which must not be.
UNSTABLE_SPREAD = 0.25

#: Printed beside a workload whose spread exceeds `UNSTABLE_SPREAD`.
UNSTABLE_MARK = "!"


@dataclass(frozen=True)
class Timing:
    """Every batch's seconds-per-call, so a figure can be read together with its uncertainty.

    `best` is the headline number and is what the previous version of this module returned. The
    rest of the samples are kept because the difference between them is the only evidence a reader
    has about whether the headline means anything.
    """

    samples: tuple[float, ...]

    @property
    def best(self) -> float:
        """The fastest batch: the closest estimate of the work, since noise only ever adds."""
        return min(self.samples)

    @property
    def worst(self) -> float:
        """The slowest batch."""
        return max(self.samples)

    @property
    def spread(self) -> float:
        """How far the slowest batch ran from the fastest, as a fraction of the fastest.

        Relative rather than absolute so it can be compared against a ratio: a workload with a
        0.73 spread cannot support a claim of 1.2x, whatever units it was measured in.
        """
        return (self.worst - self.best) / self.best if self.best > 0 else 0.0


def measure_call(call: Callable[[], object], repetitions: int = REPETITIONS) -> Timing:
    """Time `call` in `repetitions` batches and keep every one of them.

    The batch size is calibrated upward until one batch is long enough to time, so a fast workload
    and a slow one are both measured honestly rather than one of them reading as zero. The
    calibration batch is kept as the first sample — it is a real measurement of the same work at
    the same batch size, and discarding it would throw away a sample for tidiness.
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

    samples = [elapsed / batch]
    for _ in range(repetitions - 1):
        started = time.perf_counter()
        for _ in range(batch):
            call()
        samples.append((time.perf_counter() - started) / batch)
    return Timing(tuple(samples))


def per_call(call: Callable[[], object], repetitions: int = REPETITIONS) -> float:
    """Seconds per call, from the fastest of `repetitions` batches.

    Kept as the single-number form for callers that only want the headline. Anything reporting a
    comparison should use `measure_call` instead, so it can say how much to trust the number.
    """
    return measure_call(call, repetitions).best


def noise_floor(compiled: Timing, interpreted: Timing) -> float:
    """The relative difference below which this run cannot resolve anything, from the control row.

    The control is never compiled in *either* process, so its true ratio is exactly 1.0 by
    construction and every departure from 1.0 is the harness moving. Its spread is folded in
    because both sides can swing wildly while their best-of ratio sits at exactly 1.0 — reading
    only the ratio would report such a run as noiseless.
    """
    ratio = interpreted.best / compiled.best if compiled.best > 0 else 1.0
    return max(abs(ratio - 1.0), compiled.spread, interpreted.spread)


def uncertainty(floor: float, *timings: Timing) -> float:
    """The floor a particular row must clear: the run's floor, or the row's own spread if wider.

    A row can be less trustworthy than the run it sits in. `sorting.merge_sort` moves more between
    batches than the control does, so a 1.2x on that row is not a result even on a run whose
    control was steady.
    """
    return max(floor, *(timing.spread for timing in timings))


def resolves(ratio: float, floor: float) -> bool:
    """Whether a ratio is far enough from 1.0 to be a result rather than the harness moving."""
    return abs(ratio - 1.0) > floor


def format_ratio(ratio: float, floor: float) -> str:
    """A ratio as it should be printed: a figure, or the admission that there is not one."""
    return f"{ratio:.1f}x" if resolves(ratio, floor) else NOT_RESOLVABLE


def unstable(*timings: Timing) -> bool:
    """Whether any of these timings swung too far between batches to be worth reading."""
    return any(timing.spread > UNSTABLE_SPREAD for timing in timings)


def compylr_enabled() -> bool:
    """Whether this process actually compiled, asked of the manager rather than of the environment.

    The environment is what the caller *requested*; the manager is what happened. A project
    calling `initialize(enabled=False)` itself would otherwise be reported as compiled.
    """
    from ._compylr import c

    return bool(c.enabled)


def measure_in_child(module: str, arguments: list[str], *, disabled: bool) -> dict[str, Any]:
    """Run `module` as a subprocess with compilation on or off, and parse the JSON it prints.

    The child does the measuring and prints one JSON object; the parent only joins the two halves.
    That split is what keeps the comparison honest — neither process ever holds both modes.
    """
    environment = dict(os.environ)
    if disabled:
        environment[DISABLE_ENV] = "1"
    else:
        environment.pop(DISABLE_ENV, None)

    result = subprocess.run(
        [sys.executable, "-m", module, *arguments, "--emit-json"],
        capture_output=True,
        text=True,
        env=environment,
        check=False,
    )
    if result.returncode != 0:
        mode = "interpreted" if disabled else "compiled"
        raise RuntimeError(f"the {mode} run failed:\n{result.stderr}")
    measured: dict[str, Any] = json.loads(result.stdout)
    return measured
