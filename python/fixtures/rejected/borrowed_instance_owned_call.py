class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


def caller(value: Tally) -> int:
    return consume([value])


def consume(values: list[Tally]) -> int:
    return 1
