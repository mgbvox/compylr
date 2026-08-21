"""Run every algorithm, then print what the build actually exercised.

    python -m algorithms

Each line shows a call and its answer, and every answer is checked against a value computed here
in ordinary interpreted Python — usually by the standard library, which is a better oracle than a
second copy of the algorithm because it was written by someone else. A disagreement is reported
and sets the exit status, so this is a smoke test you can read as well as run.

The coverage table at the end is read off `.compylr/ir/unit.json`, the IR of the build that just
served those calls. It is the demo's claim to showcase the whole subset, in a form that can be
wrong.
"""

from __future__ import annotations

import functools
import math
import statistics
import sys
from collections import Counter
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

from . import arithmetic, dynamic, graphs, matrices, sorting, stats, structures, text
from ._compylr import c
from .ir_coverage import measure

#: A small graph used by three of the demonstrations. Acyclic, so it has a topological order.
GRAPH: dict[int, list[int]] = {0: [1, 2], 1: [3], 2: [3, 4], 3: [5], 4: [5], 5: []}

WORDS = ["the", "quick", "brown", "fox", "the", "lazy", "dog", "the", "fox"]

SAMPLE = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]


@dataclass(frozen=True)
class Demonstration:
    """One call, its answer, and what an interpreted oracle says it should be."""

    label: str
    call: Callable[[], Any]
    oracle: Callable[[], Any]


def demonstrations() -> list[Demonstration]:
    """Every algorithm, paired with an independent interpreted answer.

    The oracle is the standard library wherever one exists: `sorted`, `math.gcd`, `math.isqrt`,
    `statistics.pvariance`, `graphlib`. Hand-written references are a last resort, because a
    reference written beside the implementation tends to make the same mistake.
    """
    from graphlib import TopologicalSorter

    unsorted = [9, 1, 8, 2, 7, 3, 6, 4, 5, 1]
    left, right = [2, 4, 6, 8], [4, 8, 2, 6]
    weights, values = [1, 3, 4, 5], [1, 4, 5, 7]

    return [
        Demonstration(
            "sorting.merge_sort([9, 1, 8, ...])",
            lambda: sorting.merge_sort(unsorted),
            lambda: sorted(unsorted),
        ),
        Demonstration(
            "sorting.insertion_sort([9, 1, 8, ...])",
            lambda: sorting.insertion_sort(unsorted),
            lambda: sorted(unsorted),
        ),
        Demonstration(
            "sorting.binary_search(sorted, 7)",
            lambda: sorting.binary_search(sorted(unsorted), 7),
            lambda: sorted(unsorted).index(7),
        ),
        Demonstration(
            "arithmetic.gcd(462, 1071)",
            lambda: arithmetic.gcd(462, 1071),
            lambda: math.gcd(462, 1071),
        ),
        Demonstration(
            "arithmetic.lcm(21, 6)", lambda: arithmetic.lcm(21, 6), lambda: math.lcm(21, 6)
        ),
        Demonstration(
            "arithmetic.integer_sqrt(10**12 + 1)",
            lambda: arithmetic.integer_sqrt(10**12 + 1),
            lambda: math.isqrt(10**12 + 1),
        ),
        Demonstration("arithmetic.power(3, 25)", lambda: arithmetic.power(3, 25), lambda: 3**25),
        Demonstration(
            "arithmetic.floor_divide(-7, 2)  # Python's, not Rust's",
            lambda: arithmetic.floor_divide(-7, 2),
            lambda: -7 // 2,
        ),
        Demonstration(
            "arithmetic.remainder(-7, 2)     # signed by the divisor",
            lambda: arithmetic.remainder(-7, 2),
            lambda: -7 % 2,
        ),
        Demonstration(
            "arithmetic.sieve(50)", lambda: arithmetic.sieve(50), lambda: _primes_below(50)
        ),
        Demonstration(
            "arithmetic.to_base(255, 16)",
            lambda: arithmetic.to_base(255, 16),
            lambda: [int(digit, 16) for digit in format(255, "x")],
        ),
        Demonstration(
            "stats.mean(sample)", lambda: stats.mean(SAMPLE), lambda: statistics.fmean(SAMPLE)
        ),
        Demonstration(
            "stats.standard_deviation(sample)",
            lambda: stats.standard_deviation(SAMPLE),
            lambda: statistics.pstdev(SAMPLE),
        ),
        Demonstration(
            "stats.extremes(sample)",
            lambda: stats.extremes(SAMPLE),
            lambda: (min(SAMPLE), max(SAMPLE)),
        ),
        Demonstration(
            "text.word_count(words)",
            lambda: text.word_count(WORDS),
            lambda: dict(Counter(WORDS)),
        ),
        Demonstration(
            "text.most_common(words)",
            lambda: text.most_common(WORDS),
            # The compiled one breaks a tie by taking the alphabetically first word, because
            # iterating a mapping yields no guaranteed order. The oracle has to say the same.
            lambda: min(Counter(WORDS).items(), key=lambda pair: (-pair[1], pair[0]))[0],
        ),
        Demonstration(
            "text.unique_words(words)",
            lambda: text.unique_words(WORDS),
            lambda: list(dict.fromkeys(WORDS)),
        ),
        Demonstration(
            'text.joined(words, "-")', lambda: text.joined(WORDS, "-"), lambda: "-".join(WORDS)
        ),
        Demonstration(
            "graphs.bfs_distances(graph, 0)",
            lambda: graphs.bfs_distances(GRAPH, 0),
            lambda: {0: 0, 1: 1, 2: 1, 3: 2, 4: 2, 5: 3},
        ),
        Demonstration(
            "graphs.depth_first_order(graph, 0)",
            lambda: graphs.depth_first_order(GRAPH, 0),
            lambda: _depth_first(GRAPH, 0),
        ),
        Demonstration(
            "graphs.topological_order(graph)",
            lambda: graphs.topological_order(GRAPH),
            lambda: _one_valid_order(GRAPH, TopologicalSorter),
        ),
        Demonstration(
            "dynamic.edit_distance(...)",
            lambda: dynamic.edit_distance(["a", "b", "c"], ["a", "x", "c", "d"]),
            lambda: _levenshtein(["a", "b", "c"], ["a", "x", "c", "d"]),
        ),
        Demonstration(
            "dynamic.longest_common_subsequence(...)",
            lambda: dynamic.longest_common_subsequence(left, right),
            lambda: _lcs(left, right),
        ),
        Demonstration(
            "dynamic.coin_change([1, 5, 12], 15)",
            lambda: dynamic.coin_change([1, 5, 12], 15),
            lambda: _fewest_coins([1, 5, 12], 15),
        ),
        Demonstration(
            "dynamic.knapsack(weights, values, 7)",
            lambda: dynamic.knapsack(weights, values, 7),
            lambda: _best_load(weights, values, 7),
        ),
        Demonstration("dynamic.fibonacci(30)", lambda: dynamic.fibonacci(30), lambda: 832040),
        Demonstration(
            "matrices.multiply(a, identity)",
            lambda: matrices.multiply([[1, 2], [3, 4]], matrices.identity(2)),
            lambda: [[1, 2], [3, 4]],
        ),
        Demonstration(
            "matrices.transpose([[1, 2, 3], [4, 5, 6]])",
            lambda: matrices.transpose([[1, 2, 3], [4, 5, 6]]),
            lambda: [list(row) for row in zip(*[[1, 2, 3], [4, 5, 6]], strict=True)],
        ),
        Demonstration(
            "structures.balanced([1, 2, -2, -1])",
            lambda: structures.balanced([1, 2, -2, -1]),
            lambda: _balanced([1, 2, -2, -1]),
        ),
        Demonstration(
            "structures.component_count(6, edges)",
            lambda: structures.component_count(6, [(0, 1), (1, 2), (3, 4)]),
            lambda: _components(6, [(0, 1), (1, 2), (3, 4)]),
        ),
        Demonstration(
            "structures.RunningStats over sample",
            _running_stats,
            lambda: (len(SAMPLE), statistics.fmean(SAMPLE)),
        ),
    ]


def _lcs(left: list[int], right: list[int]) -> int:
    """The longest common subsequence, by memoised recursion.

    A different formulation from the compiled one, which fills a table bottom-up. Two versions of
    the same recurrence agreeing is weak evidence; two different shapes of it agreeing is not.
    """

    @functools.cache
    def longest(i: int, j: int) -> int:
        if i == len(left) or j == len(right):
            return 0
        if left[i] == right[j]:
            return 1 + longest(i + 1, j + 1)
        return max(longest(i + 1, j), longest(i, j + 1))

    return longest(0, 0)


def _levenshtein(left: list[str], right: list[str]) -> int:
    """The edit distance, by memoised recursion — again the other shape of the same recurrence."""

    @functools.cache
    def distance(i: int, j: int) -> int:
        if i == len(left):
            return len(right) - j
        if j == len(right):
            return len(left) - i
        if left[i] == right[j]:
            return distance(i + 1, j + 1)
        return 1 + min(distance(i + 1, j), distance(i, j + 1), distance(i + 1, j + 1))

    return distance(0, 0)


def _fewest_coins(coins: list[int], amount: int) -> int:
    """The fewest coins making `amount`, by breadth-first search over reachable totals.

    A search rather than a table, so it shares no structure at all with the compiled version.
    """
    if amount == 0:
        return 0
    seen = {0}
    frontier = [0]
    depth = 0
    while frontier:
        depth += 1
        following: list[int] = []
        for total in frontier:
            for coin in coins:
                reached = total + coin
                if reached == amount:
                    return depth
                if reached < amount and reached not in seen:
                    seen.add(reached)
                    following.append(reached)
        frontier = following
    return -1


def _best_load(weights: list[int], values: list[int], capacity: int) -> int:
    """The best knapsack value, by trying every subset.

    Exponential, and the strongest possible oracle for inputs this small: it does not implement the
    algorithm at all, it enumerates the answer space.
    """
    from itertools import combinations

    best = 0
    for count in range(len(weights) + 1):
        for chosen in combinations(range(len(weights)), count):
            if sum(weights[i] for i in chosen) <= capacity:
                best = max(best, sum(values[i] for i in chosen))
    return best


def _depth_first(graph: dict[int, list[int]], start: int) -> list[int]:
    """Visit order from a recursive depth-first search — the shape the compiled one unrolls."""
    order: list[int] = []
    seen: set[int] = set()

    def visit(node: int) -> None:
        if node in seen:
            return
        seen.add(node)
        order.append(node)
        for neighbour in graph.get(node, []):
            visit(neighbour)

    visit(start)
    return order


def _balanced(tokens: list[int]) -> bool:
    """Bracket matching over a real list-as-stack, which the subset has no `pop` for."""
    stack: list[int] = []
    for token in tokens:
        if token > 0:
            stack.append(token)
        elif not stack or stack.pop() + token != 0:
            return False
    return not stack


def _components(size: int, edges: list[tuple[int, int]]) -> int:
    """Connected components by flood fill, rather than by union-find."""
    neighbours: dict[int, list[int]] = {node: [] for node in range(size)}
    for a, b in edges:
        neighbours[a].append(b)
        neighbours[b].append(a)
    seen: set[int] = set()
    found = 0
    for node in range(size):
        if node in seen:
            continue
        found += 1
        queue = [node]
        seen.add(node)
        while queue:
            current = queue.pop()
            for neighbour in neighbours[current]:
                if neighbour not in seen:
                    seen.add(neighbour)
                    queue.append(neighbour)
    return found


def _primes_below(limit: int) -> list[int]:
    """Trial division, as an oracle for the sieve. Obvious rather than fast, on purpose."""
    return [n for n in range(2, limit) if all(n % d for d in range(2, int(n**0.5) + 1))]


def _one_valid_order(graph: dict[int, list[int]], sorter: Any) -> list[int]:
    """A topological order from the standard library, with the same tie-break the compiled one uses.

    `graphlib` takes a mapping of node to *predecessors*, which is the reverse of an adjacency
    list, and it is free to choose among ready nodes. Feeding it the reversed graph and taking
    the smallest ready node reproduces the compiled function's rule — which exists because
    iterating a mapping gives no guaranteed order, so "whichever came first" is not an answer.
    """
    predecessors: dict[int, set[int]] = {node: set() for node in graph}
    for node, neighbours in graph.items():
        for neighbour in neighbours:
            predecessors.setdefault(neighbour, set()).add(node)
    machine = sorter(predecessors)
    machine.prepare()
    order: list[int] = []
    pool: list[int] = []
    while machine.is_active():
        # `get_ready` yields each node once, so what it returns has to be accumulated rather than
        # re-read: taking only the smallest of one batch and asking again loses the rest.
        pool.extend(machine.get_ready())
        pool.sort()
        chosen = pool.pop(0)
        order.append(chosen)
        machine.done(chosen)
    return order


def _running_stats() -> tuple[int, float]:
    """Welford's algorithm folded over the sample, as (count, mean)."""
    running = structures.RunningStats()
    for value in SAMPLE:
        running.add(value)
    return running.seen(), running.mean_value()


def _agrees(answer: Any, expected: Any) -> bool:
    """Whether two answers match, allowing for float rounding.

    Floats are compared with a tolerance rather than exactly. The compiled and interpreted
    versions do the same arithmetic in the same order, so they usually agree bit for bit — but
    "usually" is not something to assert on, and a demo that fails on the last bit of a standard
    deviation teaches people to ignore it.
    """
    if isinstance(expected, float):
        return isinstance(answer, float) and math.isclose(answer, expected, rel_tol=1e-9)
    if isinstance(expected, tuple) and any(isinstance(part, float) for part in expected):
        return len(answer) == len(expected) and all(
            _agrees(a, e) for a, e in zip(answer, expected, strict=True)
        )
    return bool(answer == expected)


def _shown(value: Any) -> str:
    """A value, trimmed to something that fits on a line."""
    rendered = repr(value)
    return rendered if len(rendered) <= 44 else rendered[:41] + "..."


def main(argv: list[str] | None = None) -> int:
    """Run everything, report disagreements, and print the coverage table."""
    del argv
    print(f"algorithms, compiled: {'yes' if c.enabled else 'no (COMPYLR_DISABLE is set)'}\n")

    disagreements = 0
    for demonstration in demonstrations():
        answer = demonstration.call()
        expected = demonstration.oracle()
        print(f"  {demonstration.label:<46} {_shown(answer)}")
        if not _agrees(answer, expected):
            disagreements += 1
            print(f"  {'':<46} DISAGREES, interpreted says {_shown(expected)}")

    if c.enabled:
        module = c.ensure_built()
        print(f"\nall of it came from {module.__name__}\n")
        _report_coverage()
    else:
        # Nothing was compiled, so there is no IR to report coverage over. The answers above are
        # still checked, which is what running this way is for: it separates "compylr is wrong"
        # from "my code is wrong", without editing anything.
        print("\nran interpreted, so there is no build to report coverage over")

    if disagreements:
        print(f"\n{disagreements} disagreement(s) with the interpreted oracle", file=sys.stderr)
        return 1
    return 0


def _report_coverage() -> None:
    """Print what the build that just served those calls actually exercised."""
    coverage = measure(c.paths.ir)
    print(coverage.report())
    gaps = coverage.gaps()
    if gaps:
        print("\nnot covered:")
        for table, forms in gaps.items():
            print(f"  {table}: {', '.join(forms)}")
    else:
        print("\nEvery IR form a Python program can produce is exercised by this package.")


if __name__ == "__main__":
    raise SystemExit(main())
