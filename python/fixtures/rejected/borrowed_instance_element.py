class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


class Holder:
    def __init__(self) -> None:
        self.items: list[Tally] = []


def steal(holder: Holder) -> Tally:
    return holder.items[0]
