"""The recursive variant.

The recursion is over **primes found**, not over candidates tested. A version that recursed once per
candidate integer would reach a depth in the thousands for a modest `n`, and a stack overflow in
compiled code is a process abort rather than a recoverable error — the one failure mode that leaves
nothing at all to diagnose from. Recursing over primes keeps depth proportional to `n`.

That bound is real and is stated in the demo's README rather than left to be discovered by crashing.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def recursive_is_prime(n: int) -> bool:
    """Whether `n` is prime, by trial division."""
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d = d + 1
    return True


@c.compyle
def recursive_next_prime(after: int) -> int:
    """The smallest prime strictly greater than `after`."""
    # The loop cannot be the only exit: compylr does not assume a loop body runs, so a function
    # whose only `return` is inside one is rejected. Carrying the answer out is the shape that
    # satisfies both the compiler and a reader.
    candidate = after + 1
    found = 0
    while found == 0:
        if recursive_is_prime(candidate):
            found = candidate
        candidate = candidate + 1
    return found


@c.compyle
def recursive_nth_prime_from(remaining: int, current: int) -> int:
    """The `remaining`th prime after `current`, recursing once per prime rather than per integer."""
    if remaining < 1:
        return current
    return recursive_nth_prime_from(remaining - 1, recursive_next_prime(current))


@c.compyle
def recursive_nth_prime(n: int) -> int:
    """The `n`th prime, one-indexed. Returns 0 for `n` below one."""
    if n < 1:
        return 0
    return recursive_nth_prime_from(n, 1)


#: The variant's public name. Marked members share one namespace across a whole project, so each
#: variant's compiled functions carry a prefix and the module re-exports the readable name.
nth_prime = recursive_nth_prime
