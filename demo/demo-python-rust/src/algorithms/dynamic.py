"""Dynamic programming over two-dimensional tables.

Every algorithm here fills a `list[list[int]]`, which is where nested collections and index
assignment meet. Two rules of the subset shape all of them.

**A table has to be built before it is filled.** There is no `[[0] * n for _ in range(m)]` —
comprehensions are not in the subset and neither is `*` on a list — so `table_of_zeros` is the
loop that builds one. It is also the honest version: the comprehension allocates the same rows.

**A row read out of a table is a copy.** `row = table[i]` copies, so mutating `row` would not
reach the table. Writing through the table — `table[i][j] = v` — does, because a mutation target
is emitted as a *place* rather than as a value. That distinction is worth stating because
getting it wrong is silent: the writes go somewhere and the answer is quietly wrong.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def table_of_zeros(rows: int, columns: int) -> list[list[int]]:
    """A `rows` by `columns` table of zeros.

    Written once and called by everything below. Each row is appended as a freshly built list,
    which is not a detail: a version that built one row and appended it `rows` times would give
    every row the same identity in Python and independent rows here, so the two languages would
    disagree about what writing to one of them does.
    """
    table: list[list[int]] = []
    for _row in range(rows):
        line: list[int] = []
        for _column in range(columns):
            line.append(0)
        table.append(line)
    return table


@c.compyle
def smaller(a: int, b: int) -> int:
    """The smaller of two integers — `min` is not in the subset."""
    if a < b:
        return a
    return b


@c.compyle
def larger(a: int, b: int) -> int:
    """The larger of two integers — `max` is not in the subset."""
    if a > b:
        return a
    return b


@c.compyle
def edit_distance(left: list[str], right: list[str]) -> int:
    """The Levenshtein distance between two sequences of tokens.

    Over `list[str]` rather than over two strings, because a `str` cannot be indexed in the
    subset — see `text.py`. The algorithm is identical; only the unit of comparison moves from a
    character to a word.
    """
    rows = len(left)
    columns = len(right)
    table = table_of_zeros(rows + 1, columns + 1)
    for i in range(rows + 1):
        table[i][0] = i
    for j in range(columns + 1):
        table[0][j] = j
    for i in range(1, rows + 1):
        for j in range(1, columns + 1):
            if left[i - 1] == right[j - 1]:
                table[i][j] = table[i - 1][j - 1]
            else:
                best = smaller(table[i - 1][j], table[i][j - 1])
                table[i][j] = smaller(best, table[i - 1][j - 1]) + 1
    return table[rows][columns]


@c.compyle
def longest_common_subsequence(left: list[int], right: list[int]) -> int:
    """The length of the longest subsequence common to both lists.

    Length rather than the subsequence itself: reconstructing it walks the table backwards and
    would double the code without adding a construct the demo does not already show.
    """
    rows = len(left)
    columns = len(right)
    table = table_of_zeros(rows + 1, columns + 1)
    for i in range(1, rows + 1):
        for j in range(1, columns + 1):
            if left[i - 1] == right[j - 1]:
                table[i][j] = table[i - 1][j - 1] + 1
            else:
                table[i][j] = larger(table[i - 1][j], table[i][j - 1])
    return table[rows][columns]


@c.compyle
def coin_change(coins: list[int], amount: int) -> int:
    """The fewest coins that make `amount`, or -1 when no combination does.

    A one-dimensional table, and the place a sentinel earns its keep: "unreachable" has to be a
    value the arithmetic cannot accidentally produce, so it is `amount + 1` — one more than the
    largest number of coins any real answer could use.
    """
    if amount < 0:
        return -1
    unreachable = amount + 1
    best: list[int] = []
    for _slot in range(amount + 1):
        best.append(unreachable)
    best[0] = 0
    for target in range(1, amount + 1):
        for coin in coins:
            if coin > target:
                continue
            candidate = best[target - coin] + 1
            if candidate < best[target]:  # noqa: PLR1730 - no `min` in the subset
                best[target] = candidate
    if best[amount] == unreachable:
        return -1
    return best[amount]


@c.compyle
def knapsack(weights: list[int], values: list[int], capacity: int) -> int:
    """The greatest total value that fits in `capacity`, taking each item at most once.

    The classic 0/1 knapsack. `weights` and `values` are parallel lists because the subset has no
    record type and a `list[tuple[int, int]]` would need a tuple read per access — this reads
    better and is the shape the interpreted reference uses too.
    """
    items = len(weights)
    if items > len(values):
        return 0
    table = table_of_zeros(items + 1, capacity + 1)
    for i in range(1, items + 1):
        for room in range(capacity + 1):
            table[i][room] = table[i - 1][room]
            if weights[i - 1] > room:
                continue
            taken = table[i - 1][room - weights[i - 1]] + values[i - 1]
            if taken > table[i][room]:  # noqa: PLR1730 - no `max` in the subset
                table[i][room] = taken
    return table[items][capacity]


@c.compyle
def fibonacci(n: int) -> int:
    """The `n`th Fibonacci number, iteratively. Zero for a negative `n`.

    The bottom-up version rather than the recursive one on purpose: the recursion is exponential
    in both languages, so timing it would compare two implementations of the same waste. What is
    worth measuring is the loop.
    """
    if n < 0:
        return 0
    previous = 0
    current = 1
    step = 0
    while step < n:
        held = current
        current = previous + current
        previous = held
        step = step + 1
    return previous
