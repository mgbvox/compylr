# Some members here are named to stay distinct across the whole accepted corpus: the
# differential boundary tier builds every fixture into ONE unit, as a real project is built,
# and a duplicate name is refused by `Unit::add_function`. Renaming one back would break that
# build rather than any rule this fixture tests.
def first_and_count(xs: list[int]) -> int:
    first = xs[0]
    count = len(xs)
    return first + count


def lookup(d: dict[str, int], key: str) -> int:
    return d[key]


def build_list() -> list[int]:
    xs = [1, 2, 3]
    return xs


def mapping() -> dict[str, int]:
    return {"a": 1, "b": 2}


def unique() -> set[int]:
    return {1, 2, 2}


def pair() -> str:
    t = (1, "a")
    return t[1]


def nested(d: dict[str, list[int]], key: str) -> int:
    inner = d[key]
    return inner[0]


def from_end(xs: list[int]) -> int:
    return xs[-1]


def characters(s: str) -> int:
    return len(s)
