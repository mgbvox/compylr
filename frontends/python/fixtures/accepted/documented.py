# Some members here are named to stay distinct across the whole accepted corpus: the
# differential boundary tier builds every fixture into ONE unit, as a real project is built,
# and a duplicate name is refused by `Unit::add_function`. Renaming one back would break that
# build rather than any rule this fixture tests.
def summed(a: int, b: int) -> int:
    """Return the sum of two integers."""
    return a + b


def described(n: int) -> int:
    """Scale a value.

    A longer explanation, spanning several lines, with a blank line above.
    """
    doubled = n * 2
    return doubled


def undocumented(n: int) -> int:
    return n
