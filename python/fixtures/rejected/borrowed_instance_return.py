class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


def identity(value: Tally) -> Tally:
    return value
