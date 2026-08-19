def sign(n: int) -> int:
    if n > 0:
        return 1
    elif n < 0:
        return -1
    else:
        return 0


def clamp(n: int, low: int, high: int) -> int:
    if n < low:
        return low
    if n > high:
        return high
    return n


def describe(n: int) -> str:
    label = "small"
    if n > 100:
        label = "large"
    return label
