class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


def alias(value: Tally) -> int:
    same = value
    return same.count
