# A free function whose signature names a class: one returning an instance, one taking it.
#
# Kept apart from `classes.py` because the Python bridge cannot express this shape. It generates
# `-> PyResult<Counter>` naming the *inner* struct, where PyO3 needs the `#[pyclass]` wrapper the
# bridge built around it, and the generated crate does not compile. Nothing had ever built such a
# function through the bridge, so nothing caught it -- the differential boundary tier did, on its
# first run, and excludes this one fixture by name until the bridge learns the shape.
#
# The translation tier covers it in full: the defect is in the *boundary*, not in the translation.


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
