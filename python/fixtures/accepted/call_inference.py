def helper(n: int) -> int:
    return n


def f(a: int) -> int:
    b = helper(a)
    return b


def forward(a: int) -> int:
    # Calls a function defined below it: the signature pass sees the whole source first, so
    # definition order does not matter.
    return later(a)


def later(a: int) -> int:
    return a + 1


def promoted(a: int) -> float:
    # An integer argument where a float is declared carries an explicit conversion.
    return scale(a)


def scale(x: float) -> float:
    return x * 2.0
