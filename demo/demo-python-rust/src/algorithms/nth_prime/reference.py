"""The oracle: a plain, uncompiled nth-prime.

Deliberately the most obvious implementation rather than the fastest. Every compiled variant is
checked against this, so it earns its place by being readable — a clever oracle that is subtly
wrong makes every other test agree on the wrong answer.
"""

from __future__ import annotations


def is_prime(n: int) -> bool:
    """Whether `n` is prime, by trial division."""
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d += 1
    return True


def nth_prime(n: int) -> int:
    """The `n`th prime, one-indexed: `nth_prime(1)` is 2.

    Returns 0 for n below one, which is the edge every variant here shares. Raising would be more
    Pythonic and would make the three variants harder to compare, since the compiled subset has no
    exceptions of its own.
    """
    if n < 1:
        return 0
    found = 0
    candidate = 1
    while found < n:
        candidate += 1
        if is_prime(candidate):
            found += 1
    return candidate
