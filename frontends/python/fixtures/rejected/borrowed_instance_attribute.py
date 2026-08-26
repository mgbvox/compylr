class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start


class Holder:
    def __init__(self) -> None:
        self.item: Tally = Tally(0)


def store(holder: Holder, value: Tally) -> int:
    holder.item = value
    return holder.item.count
