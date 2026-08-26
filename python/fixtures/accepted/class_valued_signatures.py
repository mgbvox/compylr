# Free functions whose direct signatures name a class. Parameters borrow the instance held by the
# Python wrapper; returned instances are newly owned values wrapped at the boundary.


class Tally:
    def __init__(self, start: int) -> None:
        self.count: int = start

    def bump(self, by: int) -> None:
        self.count = self.count + by

    def get(self) -> int:
        return self.count


def build(start: int) -> Tally:
    t = Tally(start)
    t.bump(1)
    return t


def read(t: Tally) -> int:
    return t.count


def mutate(t: Tally, by: int) -> int:
    t.count = t.count + by
    return t.count


def mutate_method(t: Tally, by: int) -> int:
    t.bump(by)
    return read(t)


def forward_tally(t: Tally, by: int) -> int:
    return mutate_method(t, by)


def build_and_forward(start: int, by: int) -> Tally:
    t = Tally(start)
    changed = forward_tally(t, by)
    t.count = changed
    return t
