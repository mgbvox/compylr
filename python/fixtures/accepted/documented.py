def add(a: int, b: int) -> int:
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
