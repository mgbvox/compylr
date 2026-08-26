# Some members here are named to stay distinct across the whole accepted corpus: the
# differential boundary tier builds every fixture into ONE unit, as a real project is built,
# and a duplicate name is refused by `Unit::add_function`. Renaming one back would break that
# build rather than any rule this fixture tests.
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
    return scale_float(a)


def scale_float(x: float) -> float:
    return x * 2.0
