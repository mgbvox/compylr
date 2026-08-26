"""Every compiled algorithm, against an oracle that is not a copy of it.

The failure worth catching is a compiled function that runs, returns a plausible value, and is
wrong — which is what happened while this package was being written: a write through a nested
collection landed in a copy of the row, so every dynamic-programming table came back full of
zeros. Nothing raised, and the answers looked like answers.

So the oracle is **the standard library wherever one exists** — `sorted`, `bisect`, `math.gcd`,
`math.isqrt`, `statistics`, `Counter`, `graphlib` — and a differently-shaped hand-written
reference where none does. A reference written next to the implementation tends to make the same
mistake; `sorted` was written by somebody else years ago.

Inputs are seeded random as well as hand-picked. The hand-picked ones cover the edges anybody
would think of; the random ones cover the ones nobody did.
"""

from __future__ import annotations

import math
import statistics
from bisect import bisect_left
from collections import Counter
from functools import cache
from graphlib import TopologicalSorter
from itertools import combinations, pairwise
from random import Random
from typing import ClassVar

import pytest

from algorithms import arithmetic, dynamic, graphs, matrices, sorting, stats, structures, text

#: Fixed so a failure is reproducible. A test that fails once a week and passes on rerun is worse
#: than one that never runs.
SEED = 20260821


def _lists(count: int, length: int, spread: int) -> list[list[int]]:
    """`count` random integer lists, each up to `length` long."""
    source = Random(SEED)
    return [
        [source.randint(-spread, spread) for _ in range(source.randint(0, length))]
        for _ in range(count)
    ]


# ---------------------------------------------------------------------------
# Sorting and searching
# ---------------------------------------------------------------------------


class TestSorting:
    """Three sorts, one oracle: `sorted`."""

    ALGORITHMS: ClassVar = (sorting.insertion_sort, sorting.selection_sort, sorting.merge_sort)

    @pytest.mark.parametrize("sort", ALGORITHMS, ids=lambda f: f.__name__)
    @pytest.mark.parametrize("xs", _lists(count=25, length=40, spread=50))
    def test_it_agrees_with_sorted(self, sort, xs: list[int]) -> None:
        assert sort(xs) == sorted(xs)

    @pytest.mark.parametrize("sort", ALGORITHMS, ids=lambda f: f.__name__)
    @pytest.mark.parametrize(
        "xs",
        [[], [1], [1, 1], [2, 1], [1, 2, 3], [3, 2, 1], [5, 5, 5, 5], [0, -1, 1, -2, 2]],
    )
    def test_the_edges_agree_too(self, sort, xs: list[int]) -> None:
        assert sort(xs) == sorted(xs)

    @pytest.mark.parametrize("sort", ALGORITHMS, ids=lambda f: f.__name__)
    def test_it_does_not_mutate_its_argument(self, sort) -> None:
        # It could not, even by accident: a collection parameter crosses the boundary by value and
        # compylr rejects mutating one. Asserted anyway, because that is the guarantee a caller is
        # relying on and it should fail here rather than in somebody's program.
        original = [3, 1, 2]
        held = list(original)
        sort(original)
        assert original == held

    def test_is_sorted_agrees_with_a_pairwise_check(self) -> None:
        for xs in _lists(count=20, length=8, spread=3):
            expected = all(a <= b for a, b in pairwise(xs))
            assert sorting.is_sorted(xs) is expected, xs


class TestBinarySearch:
    @pytest.mark.parametrize("xs", _lists(count=20, length=30, spread=20))
    def test_it_finds_what_is_there_and_reports_what_is_not(self, xs: list[int]) -> None:
        ordered = sorted(xs)
        for target in set(ordered) | {999, -999}:
            found = sorting.binary_search(ordered, target)
            if target in ordered:
                # Any index holding the target is correct; the algorithm does not promise the
                # first one, so asserting `bisect_left` exactly would over-specify it.
                assert ordered[found] == target
            else:
                assert found == -1

    def test_the_leftmost_and_rightmost_elements_are_reachable(self) -> None:
        ordered = [1, 3, 5, 7, 9]
        assert sorting.binary_search(ordered, 1) == 0
        assert sorting.binary_search(ordered, 9) == len(ordered) - 1
        assert sorting.binary_search(ordered, 5) == bisect_left(ordered, 5)

    def test_an_empty_list_holds_nothing(self) -> None:
        assert sorting.binary_search([], 1) == -1


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


class TestArithmetic:
    PAIRS: ClassVar = [(a, b) for a in range(-12, 13) for b in range(-12, 13) if b != 0]

    @pytest.mark.parametrize(("a", "b"), PAIRS)
    def test_floor_division_and_remainder_are_pythons(self, a: int, b: int) -> None:
        # The whole reason the IR carries a rounding mode and a sign convention. Rust's native
        # operators disagree with both for exactly the operands below, and agree everywhere a
        # casual test would look.
        assert arithmetic.floor_divide(a, b) == a // b
        assert arithmetic.remainder(a, b) == a % b

    @pytest.mark.parametrize(("a", "b"), PAIRS)
    def test_the_two_stay_consistent(self, a: int, b: int) -> None:
        assert arithmetic.floor_divide(a, b) * b + arithmetic.remainder(a, b) == a

    @pytest.mark.parametrize(("a", "b"), [(462, 1071), (0, 5), (5, 0), (-12, 18), (17, 17)])
    def test_gcd_agrees_with_math(self, a: int, b: int) -> None:
        assert arithmetic.gcd(a, b) == math.gcd(a, b)

    @pytest.mark.parametrize(("a", "b"), [(21, 6), (4, 6), (0, 5), (5, 0), (-4, 6)])
    def test_lcm_agrees_with_math(self, a: int, b: int) -> None:
        assert arithmetic.lcm(a, b) == math.lcm(a, b)

    @pytest.mark.parametrize("n", [0, 1, 2, 3, 8, 9, 10, 99, 100, 10**6, 10**12 + 1])
    def test_integer_sqrt_agrees_with_math(self, n: int) -> None:
        assert arithmetic.integer_sqrt(n) == math.isqrt(n)

    def test_integer_sqrt_reports_a_negative_input(self) -> None:
        assert arithmetic.integer_sqrt(-1) == -1

    @pytest.mark.parametrize(("base", "exponent"), [(2, 0), (2, 10), (3, 25), (-2, 7), (10, 18)])
    def test_power_agrees_with_the_operator(self, base: int, exponent: int) -> None:
        assert arithmetic.power(base, exponent) == base**exponent

    def test_power_of_a_negative_exponent_is_defined(self) -> None:
        assert arithmetic.power(2, -1) == 0

    @pytest.mark.parametrize("limit", [0, 2, 3, 4, 30, 50, 200])
    def test_the_sieve_agrees_with_trial_division(self, limit: int) -> None:
        expected = [n for n in range(2, limit) if all(n % d for d in range(2, math.isqrt(n) + 1))]
        assert arithmetic.sieve(limit) == expected

    @pytest.mark.parametrize("n", [1, 2, 3, 6, 7, 27, 97])
    def test_collatz_length_agrees_with_the_rule_applied_directly(self, n: int) -> None:
        steps, current = 0, n
        while current != 1:
            current = current // 2 if current % 2 == 0 else 3 * current + 1
            steps += 1
        assert arithmetic.collatz_length(n) == steps

    def test_collatz_below_one_is_defined(self) -> None:
        assert arithmetic.collatz_length(0) == 0

    def test_both_behaviors_produce_the_documented_collatz_answer(self) -> None:
        assert arithmetic.collatz_length(97) == 118
        assert arithmetic.collatz_length_rust(97) == 118

    @pytest.mark.parametrize("n", [0, 7, 10, 99, -99, 123456789])
    def test_digit_sum_agrees_with_summing_the_string(self, n: int) -> None:
        assert arithmetic.digit_sum(n) == sum(int(d) for d in str(abs(n)))

    @pytest.mark.parametrize(("a", "b"), [(17, 5), (-17, 5), (17, -5), (0, 3)])
    def test_divide_agrees_with_divmod(self, a: int, b: int) -> None:
        assert arithmetic.divide(a, b) == divmod(a, b)

    @pytest.mark.parametrize(("n", "base"), [(0, 10), (255, 16), (255, 2), (1000, 10), (-9, 3)])
    def test_to_base_round_trips(self, n: int, base: int) -> None:
        digits = arithmetic.to_base(n, base)
        assert all(0 <= digit < base for digit in digits)
        rebuilt = 0
        for digit in digits:
            rebuilt = rebuilt * base + digit
        assert rebuilt == abs(n)

    def test_dividing_by_zero_reports_rather_than_being_undefined(self) -> None:
        # The guarantee the Python frontend requires and the Rust backend preserves, seen from the
        # calling side: it arrives as the exception Python would have raised.
        with pytest.raises(ZeroDivisionError):
            arithmetic.to_base(10, 0)


# ---------------------------------------------------------------------------
# Statistics
# ---------------------------------------------------------------------------


class TestStatistics:
    SAMPLES: ClassVar = [
        [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
        [1.5],
        [-3.0, 3.0],
        [0.25, 0.5, 0.75, 1.0],
        [1e6, 1e6 + 1, 1e6 + 2],
    ]

    @pytest.mark.parametrize("xs", SAMPLES)
    def test_mean_agrees_with_statistics(self, xs: list[float]) -> None:
        assert stats.mean(xs) == pytest.approx(statistics.fmean(xs))

    @pytest.mark.parametrize("xs", SAMPLES)
    def test_variance_and_deviation_agree_with_statistics(self, xs: list[float]) -> None:
        assert stats.variance(xs) == pytest.approx(statistics.pvariance(xs))
        assert stats.standard_deviation(xs) == pytest.approx(statistics.pstdev(xs))

    @pytest.mark.parametrize("value", [0.5, 1.0, 2.0, 10.0, 1e6, 1e-6])
    def test_square_root_agrees_with_math(self, value: float) -> None:
        assert stats.square_root(value) == pytest.approx(math.sqrt(value))

    def test_square_root_of_a_non_positive_is_defined(self) -> None:
        assert stats.square_root(0.0) == 0.0
        assert stats.square_root(-1.0) == 0.0

    @pytest.mark.parametrize("xs", SAMPLES)
    def test_extremes_agrees_with_min_and_max(self, xs: list[float]) -> None:
        assert stats.extremes(xs) == (min(xs), max(xs))

    @pytest.mark.parametrize("xs", SAMPLES)
    def test_normalize_puts_everything_between_zero_and_one(self, xs: list[float]) -> None:
        scaled = stats.normalize(xs)
        assert len(scaled) == len(xs)
        assert all(0.0 <= value <= 1.0 for value in scaled)
        if len(set(xs)) > 1:
            assert min(scaled) == pytest.approx(0.0)
            assert max(scaled) == pytest.approx(1.0)

    def test_a_constant_input_normalises_to_zeros_rather_than_dividing_by_zero(self) -> None:
        # Float division by zero does not raise — IEEE-754 says it is an infinity — so the guard
        # is the only thing keeping the answer finite.
        assert stats.normalize([3.0, 3.0, 3.0]) == [0.0, 0.0, 0.0]

    @pytest.mark.parametrize("xs", SAMPLES)
    def test_median_of_sorted_agrees_with_statistics(self, xs: list[float]) -> None:
        assert stats.median_of_sorted(sorted(xs)) == pytest.approx(statistics.median(xs))

    def test_the_empty_edges_are_defined(self) -> None:
        assert stats.mean([]) == 0.0
        assert stats.variance([]) == 0.0
        assert stats.extremes([]) == (0.0, 0.0)
        assert stats.median_of_sorted([]) == 0.0

    def test_an_integer_list_averages_as_a_float(self) -> None:
        # `/` is exact division even for two integers, and the widening is a node in the IR rather
        # than something each backend re-derives.
        assert stats.average_of_counts([1, 2]) == 1.5
        assert stats.average_of_counts([]) == 0.0


# ---------------------------------------------------------------------------
# Text
# ---------------------------------------------------------------------------


class TestText:
    WORDS: ClassVar = ["the", "quick", "brown", "fox", "the", "lazy", "dog", "the", "fox"]

    def test_word_count_agrees_with_counter(self) -> None:
        assert text.word_count(self.WORDS) == dict(Counter(self.WORDS))

    def test_word_count_of_nothing_is_empty(self) -> None:
        assert text.word_count([]) == {}

    def test_most_common_breaks_ties_alphabetically(self) -> None:
        # The tie-break is the assertion. Iterating a mapping yields keys in no guaranteed order,
        # so without a rule this would return a different word on different runs.
        assert text.most_common(self.WORDS) == "the"
        assert text.most_common(["b", "a"]) == "a"
        assert text.most_common([]) == ""

    def test_unique_words_keeps_first_seen_order(self) -> None:
        assert text.unique_words(self.WORDS) == list(dict.fromkeys(self.WORDS))

    def test_the_vowels_are_a_set(self) -> None:
        assert text.vowel_letters() == {"a", "e", "i", "o", "u"}

    def test_count_present_is_a_set_lookup(self) -> None:
        assert text.count_present(self.WORDS, {"the", "fox"}) == 5
        assert text.count_present(self.WORDS, set()) == 0

    def test_total_length_counts_code_points_not_bytes(self) -> None:
        # The three readings of `len` agree on ASCII, which is what would let a wrong one survive.
        # These do not: "é" is one code point and two UTF-8 bytes, and "𝄞" is one and four.
        assert text.total_length(self.WORDS) == sum(len(word) for word in self.WORDS)
        assert text.total_length(["é"]) == 1
        assert text.total_length(["𝄞"]) == 1
        assert text.total_length(["né", "𝄞"]) == 3

    def test_longest_takes_the_earliest_of_a_tie(self) -> None:
        assert text.longest(self.WORDS) == "quick"
        assert text.longest(["ab", "cd"]) == "ab"
        assert text.longest([]) == ""

    @pytest.mark.parametrize("separator", ["", "-", ", "])
    def test_joined_agrees_with_str_join(self, separator: str) -> None:
        assert text.joined(self.WORDS, separator) == separator.join(self.WORDS)
        assert text.joined([], separator) == ""
        assert text.joined(["only"], separator) == "only"

    def test_membership_over_a_string_tests_substrings(self) -> None:
        assert text.occurrences("a cab", ["ab", "ca", "zz"]) == 2
        assert text.missing("a cab", ["ab", "zz", "qq"]) == ["zz", "qq"]

    def test_missing_and_occurrences_partition_the_needles(self) -> None:
        needles = ["ab", "zz", "ca", "qq"]
        assert text.occurrences("a cab", needles) + len(text.missing("a cab", needles)) == len(
            needles
        )


# ---------------------------------------------------------------------------
# Graphs
# ---------------------------------------------------------------------------

#: A small acyclic graph, and a cyclic one. Both are used by several assertions.
DAG: dict[int, list[int]] = {0: [1, 2], 1: [3], 2: [3, 4], 3: [5], 4: [5], 5: []}
CYCLIC: dict[int, list[int]] = {0: [1], 1: [2], 2: [0]}


def _bfs(graph: dict[int, list[int]], start: int) -> dict[int, int]:
    """Hop counts, by a queue and `pop(0)` — the shape the compiled one cannot spell."""
    from collections import deque

    distance = {start: 0}
    queue = deque([start])
    while queue:
        node = queue.popleft()
        for neighbour in graph.get(node, []):
            if neighbour not in distance:
                distance[neighbour] = distance[node] + 1
                queue.append(neighbour)
    return distance


def _depth_first(graph: dict[int, list[int]], start: int) -> list[int]:
    """Visit order, by recursion rather than by an explicit stack."""
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


def _random_dag(source: Random, size: int) -> dict[int, list[int]]:
    """A random acyclic graph: every edge points from a lower node to a higher one."""
    graph: dict[int, list[int]] = {node: [] for node in range(size)}
    for node in range(size):
        for other in range(node + 1, size):
            if source.random() < 0.3:
                graph[node].append(other)
    return graph


class TestGraphs:
    def test_node_list_is_ascending_and_complete(self) -> None:
        # Ascending because the mapping is not: iterating it gives no guaranteed order, so the
        # sort is what makes every result downstream of this reproducible.
        found = graphs.node_list({7: [1], 3: [9]})
        assert found == [1, 3, 7, 9]

    @pytest.mark.parametrize("start", [0, 1, 5])
    def test_bfs_agrees_with_a_deque_based_reference(self, start: int) -> None:
        assert graphs.bfs_distances(DAG, start) == _bfs(DAG, start)

    def test_bfs_from_a_node_the_graph_does_not_list(self) -> None:
        # A node with no outgoing edges is still zero hops from itself.
        assert graphs.bfs_distances(DAG, 99) == {99: 0}

    @pytest.mark.parametrize("start", [0, 2])
    def test_depth_first_agrees_with_the_recursive_reference(self, start: int) -> None:
        assert graphs.depth_first_order(DAG, start) == _depth_first(DAG, start)

    def test_depth_first_terminates_on_a_cycle(self) -> None:
        assert graphs.depth_first_order(CYCLIC, 0) == [0, 1, 2]

    def test_topological_order_is_a_valid_order_graphlib_accepts(self) -> None:
        order = graphs.topological_order(DAG)
        assert sorted(order) == graphs.node_list(DAG)
        position = {node: i for i, node in enumerate(order)}
        for node, neighbours in DAG.items():
            for neighbour in neighbours:
                assert position[node] < position[neighbour], (node, neighbour)
        # And graphlib agrees the graph is orderable at all.
        TopologicalSorter({n: set() for n in order}).prepare()

    def test_topological_order_always_takes_the_smallest_ready_node(self) -> None:
        # Determinism is the property. Two runs of the same program must not differ, which
        # "whichever key the mapping offered first" would allow.
        assert graphs.topological_order(DAG) == [0, 1, 2, 3, 4, 5]
        assert graphs.topological_order({0: [], 1: [], 2: []}) == [0, 1, 2]

    def test_a_cycle_has_no_order(self) -> None:
        assert graphs.topological_order(CYCLIC) == []
        assert graphs.has_cycle(CYCLIC) is True
        assert graphs.has_cycle(DAG) is False

    @pytest.mark.parametrize("size", [1, 4, 8, 12])
    def test_random_acyclic_graphs_order_correctly(self, size: int) -> None:
        graph = _random_dag(Random(SEED + size), size)
        order = graphs.topological_order(graph)
        position = {node: i for i, node in enumerate(order)}
        assert len(order) == size
        for node, neighbours in graph.items():
            for neighbour in neighbours:
                assert position[node] < position[neighbour]
        assert graphs.has_cycle(graph) is False


# ---------------------------------------------------------------------------
# Dynamic programming
# ---------------------------------------------------------------------------


def _levenshtein(left: list[str], right: list[str]) -> int:
    """The edit distance, by memoised recursion rather than by filling a table."""

    @cache
    def distance(i: int, j: int) -> int:
        if i == len(left):
            return len(right) - j
        if j == len(right):
            return len(left) - i
        if left[i] == right[j]:
            return distance(i + 1, j + 1)
        return 1 + min(distance(i + 1, j), distance(i, j + 1), distance(i + 1, j + 1))

    return distance(0, 0)


def _lcs(left: list[int], right: list[int]) -> int:
    """The longest common subsequence, again by recursion rather than by a table."""

    @cache
    def longest(i: int, j: int) -> int:
        if i == len(left) or j == len(right):
            return 0
        if left[i] == right[j]:
            return 1 + longest(i + 1, j + 1)
        return max(longest(i + 1, j), longest(i, j + 1))

    return longest(0, 0)


def _best_load(weights: list[int], values: list[int], capacity: int) -> int:
    """The best knapsack value by enumerating every subset — not an algorithm, an answer."""
    best = 0
    for count in range(len(weights) + 1):
        for chosen in combinations(range(len(weights)), count):
            if sum(weights[i] for i in chosen) <= capacity:
                best = max(best, sum(values[i] for i in chosen))
    return best


class TestDynamicProgramming:
    """Two-dimensional tables, which is where the nested-write defect lived.

    Every one of these returned zero while `table[i][j] = v` was writing into a copy of the row.
    They are the regression tests for that as much as they are tests of the algorithms.
    """

    def test_the_zero_table_is_the_shape_asked_for(self) -> None:
        assert dynamic.table_of_zeros(2, 3) == [[0, 0, 0], [0, 0, 0]]
        assert dynamic.table_of_zeros(0, 3) == []

    def test_the_rows_of_a_zero_table_are_independent(self) -> None:
        # `[[0] * n] * m` gives every row the same identity in Python. If the compiled version did
        # the equivalent, writing one cell would write a whole column.
        table = dynamic.table_of_zeros(2, 2)
        table[0][0] = 1
        assert table == [[1, 0], [0, 0]]

    @pytest.mark.parametrize(
        ("left", "right"),
        [
            ([], []),
            (["a"], []),
            (["a", "b", "c"], ["a", "x", "c", "d"]),
            (["k", "i", "t", "t", "e", "n"], ["s", "i", "t", "t", "i", "n", "g"]),
        ],
    )
    def test_edit_distance_agrees_with_a_recursive_reference(
        self, left: list[str], right: list[str]
    ) -> None:
        assert dynamic.edit_distance(left, right) == _levenshtein(left, right)

    @pytest.mark.parametrize(
        ("left", "right"),
        [([], []), ([1], [1]), ([2, 4, 6, 8], [4, 8, 2, 6]), ([1, 2, 3, 4, 5], [3, 4, 1, 2, 5])],
    )
    def test_lcs_agrees_with_a_recursive_reference(self, left: list[int], right: list[int]) -> None:
        assert dynamic.longest_common_subsequence(left, right) == _lcs(left, right)

    def test_random_pairs_agree_for_both_table_algorithms(self) -> None:
        source = Random(SEED)
        for _ in range(15):
            left = [source.randint(0, 4) for _ in range(source.randint(0, 7))]
            right = [source.randint(0, 4) for _ in range(source.randint(0, 7))]
            assert dynamic.longest_common_subsequence(left, right) == _lcs(left, right)
            as_text_left = [str(v) for v in left]
            as_text_right = [str(v) for v in right]
            assert dynamic.edit_distance(as_text_left, as_text_right) == _levenshtein(
                as_text_left, as_text_right
            )

    @pytest.mark.parametrize(
        ("coins", "amount", "expected"),
        [([1, 5, 12], 15, 3), ([2], 3, -1), ([1], 0, 0), ([5, 10], 30, 3), ([7], -1, -1)],
    )
    def test_coin_change(self, coins: list[int], amount: int, expected: int) -> None:
        assert dynamic.coin_change(coins, amount) == expected

    @pytest.mark.parametrize("capacity", [0, 1, 5, 7, 13])
    def test_knapsack_agrees_with_enumerating_every_subset(self, capacity: int) -> None:
        weights, values = [1, 3, 4, 5], [1, 4, 5, 7]
        assert dynamic.knapsack(weights, values, capacity) == _best_load(weights, values, capacity)

    def test_knapsack_of_mismatched_lists_is_defined(self) -> None:
        assert dynamic.knapsack([1, 2], [1], 5) == 0

    @pytest.mark.parametrize("n", [*range(20), 30, 60])
    def test_fibonacci_agrees_with_the_recurrence(self, n: int) -> None:
        expected, following = 0, 1
        for _ in range(n):
            expected, following = following, expected + following
        assert dynamic.fibonacci(n) == expected

    def test_fibonacci_below_zero_is_defined(self) -> None:
        assert dynamic.fibonacci(-5) == 0

    def test_smaller_and_larger_are_min_and_max(self) -> None:
        for a, b in [(1, 2), (2, 1), (3, 3), (-1, 1)]:
            assert dynamic.smaller(a, b) == min(a, b)
            assert dynamic.larger(a, b) == max(a, b)


# ---------------------------------------------------------------------------
# Matrices
# ---------------------------------------------------------------------------


def _multiply(left: list[list[int]], right: list[list[int]]) -> list[list[int]]:
    """The matrix product, by comprehension — a different shape from the triple loop."""
    return [
        [
            sum(a * b for a, b in zip(row, column, strict=True))
            for column in zip(*right, strict=True)
        ]
        for row in left
    ]


class TestMatrices:
    A: ClassVar = [[1, 2, 3], [4, 5, 6]]
    B: ClassVar = [[7, 8], [9, 10], [11, 12]]

    def test_identity_has_ones_on_the_diagonal(self) -> None:
        assert matrices.identity(3) == [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        assert matrices.identity(0) == []

    def test_transpose_agrees_with_zip(self) -> None:
        assert matrices.transpose(self.A) == [list(row) for row in zip(*self.A, strict=True)]
        assert matrices.transpose([]) == []

    def test_transposing_twice_is_the_original(self) -> None:
        assert matrices.transpose(matrices.transpose(self.A)) == self.A

    def test_multiply_agrees_with_a_comprehension(self) -> None:
        assert matrices.multiply(self.A, self.B) == _multiply(self.A, self.B)
        assert matrices.multiply(self.B, self.A) == _multiply(self.B, self.A)

    def test_multiplying_by_the_identity_changes_nothing(self) -> None:
        assert matrices.multiply(self.A, matrices.identity(3)) == self.A
        assert matrices.multiply(matrices.identity(2), self.A) == self.A

    def test_mismatched_shapes_are_reported_as_empty(self) -> None:
        assert matrices.multiply(self.A, self.A) == []
        assert matrices.multiply([], self.A) == []

    def test_random_products_agree(self) -> None:
        source = Random(SEED)
        for _ in range(10):
            rows, inner, columns = (source.randint(1, 5) for _ in range(3))
            left = [[source.randint(-9, 9) for _ in range(inner)] for _ in range(rows)]
            right = [[source.randint(-9, 9) for _ in range(columns)] for _ in range(inner)]
            assert matrices.multiply(left, right) == _multiply(left, right)

    def test_trace_sums_the_diagonal(self) -> None:
        assert matrices.trace([[1, 2], [3, 4]]) == 5
        assert matrices.trace(self.A) == 1 + 5
        assert matrices.trace([]) == 0

    def test_row_sums_and_scale(self) -> None:
        assert matrices.row_sums(self.A) == [sum(row) for row in self.A]
        assert matrices.scale(self.A, 2) == [[v * 2 for v in row] for row in self.A]

    def test_scale_does_not_touch_its_argument(self) -> None:
        original = [[1, 2], [3, 4]]
        held = [list(row) for row in original]
        matrices.scale(original, 10)
        assert original == held


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------


class TestIntStack:
    def test_it_behaves_like_a_list_used_as_a_stack(self) -> None:
        stack = structures.IntStack()
        reference: list[int] = []
        for value in [3, 1, 4, 1, 5]:
            stack.push(value)
            reference.append(value)
            assert stack.depth() == len(reference)
            assert stack.peek() == reference[-1]
        while reference:
            assert stack.pop() == reference.pop()
        assert stack.depth() == 0

    def test_an_empty_stack_answers_rather_than_failing(self) -> None:
        # The compiled subset has no exceptions of its own, so the edge is a defined value.
        stack = structures.IntStack()
        assert stack.pop() == 0
        assert stack.peek() == 0
        assert stack.depth() == 0

    def test_two_stacks_are_independent(self) -> None:
        a, b = structures.IntStack(), structures.IntStack()
        a.push(1)
        assert a.depth() == 1
        assert b.depth() == 0

    def test_state_survives_between_calls(self) -> None:
        # The property the whole class exists for: an instance is not converted at the boundary,
        # so a mutated attribute is what the caller sees next call.
        stack = structures.IntStack()
        stack.push(7)
        assert stack.peek() == 7
        assert stack.peek() == 7


class TestBalanced:
    @pytest.mark.parametrize(
        ("tokens", "expected"),
        [
            ([], True),
            ([1, -1], True),
            ([1, 2, -2, -1], True),
            ([1, 2, -1, -2], False),
            ([1], False),
            ([-1], False),
            ([1, 1, -1, -1], True),
            ([1, 2, -2, -1, 3, -3], True),
        ],
    )
    def test_it_matches_a_list_based_reference(self, tokens: list[int], expected: bool) -> None:
        assert structures.balanced(tokens) is expected


class TestUnionFind:
    def test_it_merges_and_answers_connectivity(self) -> None:
        sets = structures.UnionFind(5)
        assert sets.group_count() == 5
        sets.union(0, 1)
        sets.union(1, 2)
        assert sets.group_count() == 3
        assert sets.connected(0, 2) is True
        assert sets.connected(0, 3) is False

    def test_merging_what_is_already_merged_changes_nothing(self) -> None:
        sets = structures.UnionFind(3)
        sets.union(0, 1)
        sets.union(0, 1)
        sets.union(1, 0)
        assert sets.group_count() == 2

    def test_path_compression_survives_the_call_that_did_it(self) -> None:
        # `find` rewrites the forest it walks. If the instance were a copy, the compression would
        # be thrown away and the structure would be a plain forest with extra steps.
        sets = structures.UnionFind(6)
        for a, b in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]:
            sets.union(a, b)
        root = sets.find(5)
        assert all(sets.find(node) == root for node in range(6))
        assert sets.group_count() == 1

    @pytest.mark.parametrize(
        ("size", "edges", "expected"),
        [
            (6, [(0, 1), (1, 2), (3, 4)], 3),
            (3, [], 3),
            (1, [], 1),
            (4, [(0, 1), (1, 2), (2, 3)], 1),
        ],
    )
    def test_component_count(self, size: int, edges: list[tuple[int, int]], expected: int) -> None:
        assert structures.component_count(size, edges) == expected

    def test_component_count_agrees_with_a_flood_fill(self) -> None:
        source = Random(SEED)
        for _ in range(10):
            size = source.randint(1, 10)
            edges = [
                (source.randrange(size), source.randrange(size))
                for _ in range(source.randint(0, size))
            ]
            neighbours: dict[int, list[int]] = {node: [] for node in range(size)}
            for a, b in edges:
                neighbours[a].append(b)
                neighbours[b].append(a)
            seen: set[int] = set()
            expected = 0
            for node in range(size):
                if node in seen:
                    continue
                expected += 1
                queue = [node]
                seen.add(node)
                while queue:
                    current = queue.pop()
                    for neighbour in neighbours[current]:
                        if neighbour not in seen:
                            seen.add(neighbour)
                            queue.append(neighbour)
            assert structures.component_count(size, edges) == expected


class TestRunningStats:
    SAMPLE: ClassVar = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]

    def test_it_agrees_with_the_batch_versions(self) -> None:
        running = structures.RunningStats()
        for value in self.SAMPLE:
            running.add(value)
        assert running.seen() == len(self.SAMPLE)
        assert running.mean_value() == pytest.approx(statistics.fmean(self.SAMPLE))
        assert running.variance_value() == pytest.approx(statistics.pvariance(self.SAMPLE))

    def test_it_agrees_at_every_prefix(self) -> None:
        # Streaming is the point, so the answer has to be right after each observation rather
        # than only at the end.
        running = structures.RunningStats()
        for index, value in enumerate(self.SAMPLE, start=1):
            running.add(value)
            prefix = self.SAMPLE[:index]
            assert running.seen() == index
            assert running.mean_value() == pytest.approx(statistics.fmean(prefix))
            assert running.variance_value() == pytest.approx(statistics.pvariance(prefix))

    def test_an_empty_stream_is_defined(self) -> None:
        running = structures.RunningStats()
        assert running.seen() == 0
        assert running.mean_value() == 0.0
        assert running.variance_value() == 0.0
