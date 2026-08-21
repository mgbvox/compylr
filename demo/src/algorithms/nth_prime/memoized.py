"""The memoized variant.

A class, because a cache has to outlive the call that fills it and the subset has no module-level
state. The hit counter exists so a test can assert the cache is *used* rather than merely present:
without it, "memoized" is a claim about the code's shape, and a refactor that broke caching would
still pass every agreement test.

An attribute can be a cache precisely because an instance is not converted at the boundary — the
Python object holds the Rust value, so a mutated attribute is what the caller sees next call. A
collection *parameter* is a copy and could not do this.
"""

from __future__ import annotations

from .._compylr import c


@c.compyle
class PrimeCache:
    """An nth-prime that remembers what it has already computed."""

    def __init__(self) -> None:
        self.known: dict[int, int] = {}
        self.hits: int = 0

    def is_prime(self, n: int) -> bool:
        if n < 2:
            return False
        d = 2
        while d * d <= n:
            if n % d == 0:
                return False
            d = d + 1
        return True

    def nth(self, n: int) -> int:
        """The `n`th prime, one-indexed. Returns 0 for `n` below one."""
        if n < 1:
            return 0
        if n in self.known:
            self.hits = self.hits + 1
            return self.known[n]
        found = 0
        candidate = 1
        while found < n:
            candidate = candidate + 1
            if self.is_prime(candidate):
                found = found + 1
        self.known[n] = candidate
        return candidate

    def hit_count(self) -> int:
        """How many requests were answered from the cache."""
        return self.hits

    def known_count(self) -> int:
        """How many answers are cached."""
        return len(self.known)
