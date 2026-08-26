class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


def replace(value: Tally) -> int:
    value = Tally(1)
    return value.count
