"""Run all three variants and show that they agree.

    python -m nth_prime 25
"""

from __future__ import annotations

import sys
import time
from collections.abc import Callable

from . import iterative, memoized, recursive, reference


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    n = int(args[0]) if args else 10

    cache = memoized.PrimeCache()
    results: list[tuple[str, Callable[[int], int]]] = [
        ("reference (interpreted)", reference.nth_prime),
        ("recursive (compiled)", recursive.nth_prime),
        ("iterative (compiled)", iterative.nth_prime),
        ("memoized (compiled)", cache.nth),
    ]

    print(f"the {n}th prime, four ways:")
    answers = []
    for label, implementation in results:
        started = time.perf_counter()
        answer = implementation(n)
        elapsed = time.perf_counter() - started
        answers.append(answer)
        print(f"  {label:<24} {answer:>8}   {elapsed * 1000:.3f} ms")

    cache.nth(n)
    print(f"\nthe cache answered {cache.hit_count()} request(s) without recomputing")

    if len(set(answers)) != 1:
        print("\nDISAGREEMENT: the variants did not all return the same answer", file=sys.stderr)
        return 1
    print("all four agree")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
