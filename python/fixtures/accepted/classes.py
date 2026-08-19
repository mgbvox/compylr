class Counter:
    """Mutable state scoped to an object, which a free function cannot hold."""

    def __init__(self, start: int) -> None:
        self.count: int = start

    def bump(self, by: int) -> None:
        self.count = self.count + by

    def bump_twice(self, by: int) -> None:
        self.bump(by)
        self.bump(by)

    def get(self) -> int:
        return self.count


class Cache:
    def __init__(self) -> None:
        self.entries: dict[int, int] = {}
        self.log: list[int] = []

    def put(self, k: int, v: int) -> None:
        self.entries[k] = v
        self.log.append(k)

    def has(self, k: int) -> bool:
        return k in self.entries

    def size(self) -> int:
        return len(self.entries)


def build(start: int) -> Counter:
    c = Counter(start)
    c.bump(1)
    return c


def read(c: Counter) -> int:
    return c.count
