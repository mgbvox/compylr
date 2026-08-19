def f() -> int:
    t: tuple[int, int] = (1, 2)
    t[0] = 3
    return t[0]
