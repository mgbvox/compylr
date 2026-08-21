"""Timing, and the two-process arrangement that makes a compiled-against-interpreted number honest.

Shared by both benchmarks in this project, deliberately. They measure different things — one
problem three ways, and forty algorithms once each — but the *method* has to be identical, or the
two sets of numbers cannot be read side by side. Two copies of it would drift.

Three decisions are load-bearing, and each is a way of being wrong that looks like a result:

**The two modes run in separate processes.** A marked function reaches other marked functions
through module globals, so an "interpreted" outer call in a compiled process would still land in
compiled code. `COMPYLR_DISABLE=1` is what makes an interpreted run interpreted all the way down.

**A batch is timed, not a call.** `perf_counter` resolves to tens of nanoseconds, and plenty of
what is measured here takes hundreds. Timing one call would report zero or one tick.

**The best of several batches is kept, not the mean.** Noise only ever adds, so the minimum is the
closest estimate of the work itself. A mean mostly measures how busy the machine was.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from collections.abc import Callable
from typing import Any

__all__ = [
    "DISABLE_ENV",
    "MINIMUM_BATCH_SECONDS",
    "REPETITIONS",
    "compylr_enabled",
    "measure_in_child",
    "per_call",
]

#: How many times each workload is timed. The best is kept.
REPETITIONS = 5

#: Environment variable that turns compilation off for a whole process.
#:
#: Named here rather than imported from compylr, so this module can talk about the switch before
#: deciding whether to import the package it belongs to.
DISABLE_ENV = "COMPYLR_DISABLE"

#: A timed batch must last at least this long to mean anything.
MINIMUM_BATCH_SECONDS = 0.01


def per_call(call: Callable[[], object], repetitions: int = REPETITIONS) -> float:
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
