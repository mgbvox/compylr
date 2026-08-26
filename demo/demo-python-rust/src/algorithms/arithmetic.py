"""Integer algorithms.

This is where the arithmetic the IR *declares* is visible. `//` and `%` are not one operation
each: the IR carries a rounding mode on division and a sign convention on remainder, and the
Python frontend sets both to Python's readings. Rust's own `/` truncates toward zero and its `%`
takes the sign of the dividend, so `-7 // 2` is `-4` here and would be `-3` from a translation
that emitted the operator positionally. `gcd` below normalises its arguments first and so never
meets the difference; `floor_divide` exists precisely to meet it.

Integer overflow is **reported**, not wrapped. `power` is written to avoid one final squaring
that would overflow for exponents whose result does not — a real bug in the obvious version, and
one that surfaces here as an exception rather than as a wrong answer.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def floor_divide(a: int, b: int) -> int:
    """`a // b`, rounding toward negative infinity as Python does.

    One line, and the reason the IR carries a rounding mode: `-7 // 2` is `-4`, where Rust's
    native `/` gives `-3`. Nothing about the operator's *name* says which, so a backend that
    matched on the name would be silently wrong for a frontend that meant the other one.
    """
    return a // b


@c.compyle
def remainder(a: int, b: int) -> int:
    """`a % b`, taking the sign of the **divisor** as Python does.

    `-7 % 2` is `1` here and `-1` in Rust. The companion to `floor_divide`, and the two are
    consistent: `(a // b) * b + a % b == a` under either convention, but only one of them agrees
    with the interpreted original.
    """
    return a % b


@c.compyle
def gcd(a: int, b: int) -> int:
    """The greatest common divisor, by Euclid's algorithm.

    Both arguments are made non-negative first, which is why this never depends on how `%` signs
    its result — `floor_divide` and `remainder` are where that shows.
    """
    x = a
    y = b
    if x < 0:
        x = -x
    if y < 0:
        y = -y
    while y != 0:
        held = y
        y = x % y
        x = held
    return x


@c.compyle
def lcm(a: int, b: int) -> int:
    """The least common multiple. Zero when either argument is zero, as `math.lcm` gives."""
    if a == 0:
        return 0
    if b == 0:
        return 0
    product = a * b
    if product < 0:
        product = -product
    return product // gcd(a, b)


@c.compyle
def integer_sqrt(n: int) -> int:
    """The floor of the square root of `n`, by Newton's method. -1 for a negative `n`.

    Integer throughout rather than a float square root and a truncation: the float version is
    wrong for large `n`, because a 64-bit float cannot represent every integer this can.
    """
    if n < 0:
        return -1
    if n < 2:
        return n
    x = n
    y = (x + 1) // 2
    while y < x:
        x = y
        y = (x + n // x) // 2
    return x


@c.compyle
def power(base: int, exponent: int) -> int:
    """`base` raised to `exponent`, by squaring. Zero for a negative exponent.

    The subset has no `**`. Note the guard before the squaring: the obvious loop squares `base`
    once more than it needs, and for an exponent near the top of the range that overflows even
    when the answer does not. Overflow is reported here rather than wrapping, so the obvious
    version fails loudly on inputs this one answers.
    """
    if exponent < 0:
        return 0
    result = 1
    factor = base
    remaining = exponent
    while remaining > 0:
        if remaining % 2 == 1:
            result = result * factor
        remaining = remaining // 2
        if remaining > 0:
            factor = factor * factor
    return result


@c.compyle
def collatz_length(n: int) -> int:
    """How many steps `n` takes to reach 1 under the Collatz rule. Zero for `n` below one.

    Nobody has proved this terminates for every `n`. It is in the demo because it is the shortest
    honest example of a loop whose trip count is not a function of its input's size.
    """
    if n < 1:
        return 0
    steps = 0
    current = n
    while current != 1:
        if current % 2 == 0:
            current = current // 2
        else:
            current = 3 * current + 1
        steps = steps + 1
    return steps


@c.compyle(behavior="rust")
def collatz_length_rust(n: int) -> int:
    """`collatz_length` again, byte for byte, compiled under Rust's meanings instead.

    The duplication is the experiment. Two functions with identical bodies and different
    `behavior` settings differ in exactly one thing — what `%`, `//`, `*` and `+` are allowed to
    do — so the benchmark's comparison between them is a comparison of behaviors and not of
    programs. What the Rust stance gives up here is written down in the demo's README.
    """
    if n < 1:
        return 0
    steps = 0
    current = n
    while current != 1:
        if current % 2 == 0:
            current = current // 2
        else:
            current = 3 * current + 1
        steps = steps + 1
    return steps


@c.compyle
def digit_sum(n: int) -> int:
    """The sum of the decimal digits of `n`, ignoring its sign."""
    current = n
    if current < 0:
        current = -current
    total = 0
    while current > 0:
        total = total + current % 10
        current = current // 10
    return total


@c.compyle
def sieve(limit: int) -> list[int]:
    """Every prime below `limit`, by the sieve of Eratosthenes.

    Two `continue`s, both load-bearing, and the reason this is in the demo: `continue` inside a
    `for` over a `range` used to skip the loop's cursor increment and hang. It was found by the
    compiler's own conformance corpus rather than by a test written in Python, which is why that
    corpus is checked over `(statement, position)` pairs instead of statements alone.
    """
    if limit < 3:
        return []
    composite: list[bool] = []
    for _slot in range(limit):
        composite.append(False)
    candidate = 2
    while candidate * candidate < limit:
        if composite[candidate]:
            candidate = candidate + 1
            continue
        multiple = candidate * candidate
        while multiple < limit:
            composite[multiple] = True
            multiple = multiple + candidate
        candidate = candidate + 1
    primes: list[int] = []
    for n in range(2, limit):
        if composite[n]:
            continue
        primes.append(n)
    return primes


@c.compyle
def divide(a: int, b: int) -> tuple[int, int]:
    """Quotient and remainder together — Python's `divmod`.

    A tuple is the subset's only heterogeneous value, and the only way a compiled function
    returns two things. Its positions are typed independently, so reading one is resolved at
    compile time: `pair[i]` for a computed `i` is rejected, because the result's type would
    depend on a runtime value.
    """
    return (a // b, a % b)


@c.compyle
def to_base(n: int, base: int) -> list[int]:
    """The digits of `n` in `base`, most significant first. Negative `n` is treated as positive.

    A `base` of zero divides by zero, which is **reported** rather than being undefined: the
    guarantee the Python frontend requires is that division by zero raises, and the Rust backend
    preserves it. The exception surfaces on the Python side as `ZeroDivisionError`.
    """
    current = n
    if current < 0:
        current = -current
    backwards: list[int] = []
    if current == 0:
        backwards.append(0)
    while current > 0:
        split = divide(current, base)
        backwards.append(split[1])
        current = split[0]
    digits: list[int] = []
    index = len(backwards) - 1
    while index >= 0:
        digits.append(backwards[index])
        index = index - 1
    return digits
