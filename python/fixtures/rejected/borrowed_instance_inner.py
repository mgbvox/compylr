class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


class Holder:
    def __init__(self) -> None:
        self.item: Tally = Tally(0)


def steal(holder: Holder) -> Tally:
    return holder.item
