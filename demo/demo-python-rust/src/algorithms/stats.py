"""Descriptive statistics, and the float half of the subset.

Two things are on display. **`/` always produces a float**, even for two integers, and the
widening is a node in the IR rather than something each backend re-derives — a translation that
emitted the operands positionally would produce integer division where Python produces float
division, and the two agree on exactly the inputs a test suite is likely to use.

And **there is no `math`**. A compiled function calls functions in the same unit and nothing
else, so a square root is Newton's method written out. That is the honest shape of the
constraint: compylr compiles your code, it does not give you a standard library.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def mean(xs: list[float]) -> float:
    """The arithmetic mean. Zero for an empty list.

    Zero rather than an exception, for the reason every edge in this demo returns a sentinel:
    the compiled subset has no exceptions of its own, so the alternative is not a better error
    but no answer at all.
    """
    if len(xs) == 0:
        return 0.0
    total = 0.0
    for x in xs:
        total = total + x
    return total / len(xs)


@c.compyle
def average_of_counts(counts: list[int]) -> float:
    """The mean of a list of **integers**, as a float.

    The one-line demonstration that `/` is exact division: `total` and `len(counts)` are both
    integers, and the result is not. Both operands are widened, and the widening is visible in
    `.compylr/ir/unit.json` as a `ToFloat` node wrapping each of them.
    """
    if len(counts) == 0:
        return 0.0
    total = 0
    for count in counts:
        total = total + count
    return total / len(counts)


@c.compyle
def variance(xs: list[float]) -> float:
    """The population variance — the mean of the squared deviations. Zero for an empty list.

    Two passes rather than the one-pass sum-of-squares identity. The identity is faster and
    loses catastrophic precision when the mean is large relative to the spread, which is exactly
    the case where somebody would trust a compiled answer more than an interpreted one.
    """
    if len(xs) == 0:
        return 0.0
    centre = mean(xs)
    total = 0.0
    for x in xs:
        deviation = x - centre
        total = total + deviation * deviation
    return total / len(xs)


@c.compyle
def square_root(value: float) -> float:
    """The square root of `value`, by Newton's method. Zero for a negative input.

    Forty iterations rather than a convergence test: the loop count is then a constant, which
    makes this a fair thing to benchmark, and forty is far past the point where a 64-bit float
    stops changing.
    """
    if value <= 0.0:
        return 0.0
    guess = value
    step = 0
    while step < 40:
        guess = (guess + value / guess) / 2.0
        step = step + 1
    return guess


@c.compyle
def standard_deviation(xs: list[float]) -> float:
    """The population standard deviation."""
    return square_root(variance(xs))


@c.compyle
def extremes(xs: list[float]) -> tuple[float, float]:
    """The smallest and largest values, together. `(0.0, 0.0)` for an empty list.

    `min` and `max` are not in the subset — only `len` and `range` are builtins — so this is the
    loop they would have hidden. Returning both from one pass is what the tuple is for.
    """
    if len(xs) == 0:
        return (0.0, 0.0)
    smallest = xs[0]
    largest = xs[0]
    for x in xs:
        if x < smallest:  # noqa: PLR1730 - no `min` in the subset
            smallest = x
        if x > largest:  # noqa: PLR1730 - no `max` in the subset
            largest = x
    return (smallest, largest)


@c.compyle
def normalize(xs: list[float]) -> list[float]:
    """`xs` rescaled so its smallest value is 0.0 and its largest is 1.0.

    A constant input has no span to divide by, and maps to all zeros rather than dividing by
    zero. Float division by zero is the one arithmetic hazard here that does **not** raise —
    IEEE-754 says it is an infinity — so guarding is the only thing that keeps the answer finite.
    """
    span = extremes(xs)
    lowest = span[0]
    highest = span[1]
    out: list[float] = []
    if highest - lowest == 0.0:
        for _x in xs:
            out.append(0.0)
        return out
    for x in xs:
        out.append((x - lowest) / (highest - lowest))
    return out


@c.compyle
def median_of_sorted(xs: list[float]) -> float:
    """The median of an already-ascending list. Zero for an empty list.

    Takes its input sorted rather than sorting it: `sorting.py` sorts integers, and a second
    sort for floats would be the same algorithm twice. The precondition is the honest trade.
    """
    count = len(xs)
    if count == 0:
        return 0.0
    if count % 2 == 1:
        return xs[count // 2]
    return (xs[count // 2 - 1] + xs[count // 2]) / 2.0
