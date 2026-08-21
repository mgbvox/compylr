"""Common algorithms, compiled.

The demo is one package with two halves that pull in opposite directions.

**Breadth** is the modules beside this file: forty-odd algorithms anybody would recognise, chosen
so that between them they reach **every** form the IR can hold — each statement, each expression,
each type, each operator, and both of the division modes a Python program can produce. That is a
checked claim, not a blurb: `ir_coverage` reads the IR compylr writes on every build and reports
what is covered, and `tests/test_coverage.py` fails when something stops being.

**Depth** is `nth_prime`: one problem, three implementations, asserted to agree with an
interpreted oracle and with each other, and then measured compiled against interpreted in two
separate processes so the number means something.

    from algorithms import arithmetic, sorting

    sorting.merge_sort([5, 3, 1])     # [1, 3, 5]
    arithmetic.sieve(30)              # [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]

    python -m algorithms                              # run everything, then the coverage table
    python -m algorithms.nth_prime 25                 # the three variants
    python -m algorithms.benchmark                    # compiled against interpreted

| module | what it is for |
| --- | --- |
| `sorting` | insertion, selection, and merge sort; binary search |
| `arithmetic` | gcd, lcm, integer square root, exponentiation, the sieve, base conversion |
| `stats` | mean, variance, standard deviation, normalisation — the float half |
| `text` | word frequencies and membership, and the limits of `str` in the subset |
| `graphs` | breadth-first distances, depth-first order, topological sort |
| `dynamic` | edit distance, longest common subsequence, coin change, knapsack |
| `matrices` | multiply, transpose, trace — the best case for compiling |
| `structures` | a stack, a union-find, and streaming statistics: state that outlives a call |
| `nth_prime` | the same problem three ways, benchmarked |

Every marked member in every one of them marks against the single manager in `_compylr`, so the
whole demo compiles into **one** extension — which is the arrangement compylr is built around,
and what lets a function in `graphs` call one in `sorting`.

Run `compylr compyle src` from the project root first, or the first call builds and is slow.
"""

from __future__ import annotations

from . import (
    arithmetic,
    dynamic,
    graphs,
    ir_coverage,
    matrices,
    nth_prime,
    sorting,
    stats,
    structures,
    text,
)
from ._compylr import c

__all__ = [
    "arithmetic",
    "c",
    "dynamic",
    "graphs",
    "ir_coverage",
    "matrices",
    "nth_prime",
    "sorting",
    "stats",
    "structures",
    "text",
]
