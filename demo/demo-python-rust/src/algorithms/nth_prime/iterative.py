"""The iterative variant.

Computes independently: a loop, a reassigned counter, and a locally built collection of the primes
found so far, which it divides by rather than trial-dividing every integer. Delegating to another
variant would make the agreement tests prove nothing.

The collection is built locally and returned by value, which is the shape compylr's mutation rule
exists to enable — a collection *parameter* is a copy and cannot be mutated.
"""

from __future__ import annotations

from .._compylr import c


@c.compyle
def iterative_primes_up_to_count(n: int) -> list[int]:
    """The first `n` primes, in order."""
    found: list[int] = []
    candidate = 2
    while len(found) < n:
        divisible = False
        for p in found:
            if p * p > candidate:
                break
            if candidate % p == 0:
                divisible = True
                break
        if iterative_not_divisible(divisible):
            found.append(candidate)
        candidate = candidate + 1
    return found


@c.compyle
def iterative_not_divisible(divisible: bool) -> bool:
    """Negation, written out because the subset has no `not` operator.

    `return not divisible` is what anyone would write, and is rejected. Recorded in the README as a
    gap this demo found rather than worked around silently.
    """
    if divisible:  # noqa: SIM103 - `not` is not in the compiled subset
        return False
    return True


@c.compyle
def iterative_nth_prime(n: int) -> int:
    """The `n`th prime, one-indexed. Returns 0 for `n` below one."""
    if n < 1:
        return 0
    found = iterative_primes_up_to_count(n)
    return found[n - 1]


#: See recursive.py: marked names are shared across a project, so the compiled functions are
#: prefixed and the readable name is re-exported here.
nth_prime = iterative_nth_prime
