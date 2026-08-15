def double(n: int) -> int:
    return n * 2


def quadruple(n: int) -> int:
    return double(double(n))


def nested_expression(a: int, b: int) -> int:
    total: int = double(a) + double(b)
    return total
