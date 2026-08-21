"""One problem, three ways — the depth half of this demo.

The package around this one goes for breadth: forty-odd algorithms chosen so that between them
they reach every form the IR can hold. This subpackage does the opposite. It takes a single
problem — the `n`th prime — implements it three ways, and asserts that all three agree with a
plain interpreted reference and with each other. Then it measures them honestly.

    from algorithms.nth_prime import iterative, memoized, recursive

    recursive.nth_prime(10)          # 29
    iterative.nth_prime(10)          # 29
    memoized.PrimeCache().nth(10)    # 29

    python -m algorithms.nth_prime 25                 # run all three
    python -m algorithms.nth_prime.benchmark --n 500  # compiled against interpreted

`reference.nth_prime` is never compiled. It is the oracle, so it is written to be obvious rather
than fast, and every other variant is checked against it.

The three mark against `algorithms._compylr`, the manager the whole demo shares, so they compile
into the same extension as everything else — which is the arrangement compylr is built around.
"""

from __future__ import annotations

from .. import _compylr
from . import iterative, memoized, recursive, reference

#: The demo's one manager, re-exported so this subpackage reads like a package rather than like a
#: fragment of one. It is the same object as `algorithms.c`.
c = _compylr.c

__all__ = ["c", "iterative", "memoized", "recursive", "reference"]
