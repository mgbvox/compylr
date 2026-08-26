# Some members here are named to stay distinct across the whole accepted corpus: the
# differential boundary tier builds every fixture into ONE unit, as a real project is built,
# and a duplicate name is refused by `Unit::add_function`. Renaming one back would break that
# build rather than any rule this fixture tests.
def add(a: int, b: int) -> int:
    return a + b


def difference(a: int, b: int) -> int:
    return a - b


def product(a: int, b: int) -> int:
    return a * b


def halve(a: int) -> int:
    return a // 2


def modulo(a: int, b: int) -> int:
    return a % b


def negate(a: int) -> int:
    return -a
