def total(n: int) -> int:
    running = 0
    for i in range(n):
        running = running + i
    return running


def countdown(n: int) -> int:
    steps = 0
    for i in range(n, 0, -1):
        steps = steps + 1
    return steps


def stepped(n: int) -> int:
    seen = 0
    for i in range(0, n, 2):
        seen = seen + i
    return seen


def first_over(xs: list[int], limit: int) -> int:
    for x in xs:
        if x > limit:
            return x
    return -1


def skip_negatives(xs: list[int]) -> int:
    kept = 0
    for x in xs:
        if x < 0:
            continue
        kept = kept + 1
    return kept


def key_lengths(d: dict[str, int]) -> int:
    total_length = 0
    for k in d:
        total_length = total_length + len(k)
    return total_length


def halve_until_odd(n: int) -> int:
    while n % 2 == 0:
        if n == 0:
            break
        n = n // 2
    return n
