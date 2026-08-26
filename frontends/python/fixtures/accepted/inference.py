def literals() -> str:
    a = "x"
    b = 3
    c = 1.3
    d = True
    return a


def expressions(n: int) -> int:
    doubled = n * 2
    shifted = doubled + 1
    return shifted


def comparisons(n: int) -> bool:
    big = n > 100
    return big
