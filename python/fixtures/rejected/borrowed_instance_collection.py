class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


def collect(value: Tally) -> int:
    values: list[Tally] = [value]
    return 1
