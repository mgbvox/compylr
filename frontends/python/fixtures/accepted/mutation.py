def evens_below(limit: int) -> list[int]:
    found: list[int] = []
    for n in range(limit):
        if n % 2 == 0:
            found.append(n)
    return found


def replace_first(n: int) -> list[int]:
    xs: list[int] = [0, 0, 0]
    xs[0] = n
    return xs


def counts(words: list[str]) -> dict[str, int]:
    seen: dict[str, int] = {}
    for word in words:
        if word in seen:
            seen[word] = seen[word] + 1
        else:
            seen[word] = 1
    return seen


def has_element(xs: list[int], x: int) -> bool:
    return x in xs


def has_key(d: dict[str, int], k: str) -> bool:
    return k in d


def has_member(s: set[int], x: int) -> bool:
    return x in s


def has_substring(hay: str, needle: str) -> bool:
    return needle in hay


def lacks_element(xs: list[int], x: int) -> bool:
    return x not in xs
