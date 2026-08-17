def increment(n: int) -> int:
    n = n + 1
    return n


def accumulate(limit: int) -> int:
    i = 0
    total = 0
    while i < limit:
        total = total + i
        i = i + 1
    return total


def widened() -> float:
    x: float = 1.0
    x = 2
    return x
