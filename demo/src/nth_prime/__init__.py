"""Three nth-prime implementations that each compile to Rust through compylr.

    from nth_prime import iterative, memoized, recursive

    recursive.nth_prime(10)          # 29
    iterative.nth_prime(10)          # 29
    memoized.PrimeCache().nth(10)    # 29

All three agree with `reference.nth_prime`, which is plain interpreted Python and exists to be the
oracle. They compile into **one** shared extension, because they share one manager.

Run `compylr compyle .` from this directory first, or the first call builds and is slow.
"""

from __future__ import annotations

from . import iterative, memoized, recursive, reference
from ._compylr import c

__all__ = ["c", "iterative", "memoized", "recursive", "reference"]
