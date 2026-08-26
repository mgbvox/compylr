"""Data structures — the part of the subset that free functions over values cannot reach.

A class is how state outlives a call. The contrast worth holding on to, because it is the thing
people get backwards:

* A collection **parameter** crosses the boundary **by value**. The callee gets a copy, mutating
  it could never be observed by the caller, and compylr rejects the attempt rather than compiling
  a program whose interpreted and compiled versions disagree.
* An **instance** is not converted at all. The Python object holds the Rust value through
  `#[pyclass]` and a method borrows it from there, so a mutated attribute **is** what the caller
  sees on the next call. That is what makes an attribute a cache, an accumulator, or a union-find
  forest.

Attributes are declared in `__init__` with mandatory annotations and nowhere else — otherwise the
struct's fields would depend on which methods happened to run. And a method's receiver is derived
by fixpoint: it is `&mut self` when the method assigns an attribute, mutates a collection
attribute, **or calls a method that does**. `UnionFind.connected` below only asks a question, and
still takes a mutable receiver, because `find` compresses the path it walks.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
class IntStack:
    """A stack of integers, over a list that never shrinks.

    `append` is the only collection method in the subset — there is no `pop` — so the stack is a
    list and a `height`. Pushing writes over a slot past the height when one is there and appends
    otherwise, and popping just lowers the height. The list therefore grows to the deepest the
    stack ever was and stays there, which is the allocation `pop` would have handed back and that
    a stack is about to want again anyway.
    """

    def __init__(self) -> None:
        self.slots: list[int] = []
        self.height: int = 0

    def push(self, value: int) -> None:
        """Put `value` on top."""
        if self.height < len(self.slots):
            self.slots[self.height] = value
        else:
            self.slots.append(value)
        self.height = self.height + 1

    def pop(self) -> int:
        """Take the top value off, or 0 when the stack is empty."""
        if self.height == 0:
            return 0
        self.height = self.height - 1
        return self.slots[self.height]

    def peek(self) -> int:
        """The top value without removing it, or 0 when the stack is empty."""
        if self.height == 0:
            return 0
        return self.slots[self.height - 1]

    def depth(self) -> int:
        """How many values are on the stack."""
        return self.height


@c.compyle
def balanced(tokens: list[int]) -> bool:
    """Whether the markers in `tokens` open and close in the right order.

    A positive token opens, and its negative closes it: `[1, 2, -2, -1]` is balanced and
    `[1, 2, -1, -2]` is not. Integers rather than characters because a `str` cannot be indexed or
    iterated in the subset, so a bracket string would have to be tokenised by the caller anyway.

    `stack.push(token)` is a statement rather than an expression, which the subset allows only
    because `push` returns `None`. Discarding a *value* is rejected — it is either dead code or a
    side effect the subset cannot express — so a method whose result you mean to ignore has to
    say so in its return type.
    """
    stack = IntStack()
    for token in tokens:
        if token > 0:
            stack.push(token)
        else:
            if stack.depth() == 0:
                return False
            if stack.pop() + token != 0:
                return False
    return stack.depth() == 0


@c.compyle
class UnionFind:
    """Disjoint sets, with union by rank and path compression.

    The structure that makes Kruskal's algorithm and connected-components linear-ish, and the
    best small example of why an instance is not a copy: `find` **rewrites the forest it walks**,
    and that rewrite has to still be there on the next call or the compression bought nothing.
    """

    def __init__(self, size: int) -> None:
        self.parent: list[int] = []
        self.rank: list[int] = []
        self.groups: int = size
        for node in range(size):
            self.parent.append(node)
            self.rank.append(0)

    def find(self, node: int) -> int:
        """The representative of `node`'s set, flattening the path to it on the way.

        Two passes rather than recursion: the root is found first, then every node on the way is
        pointed straight at it. Recursion would be shorter and would put the depth of the tree on
        the call stack, and a stack overflow in compiled code is a process abort with no
        traceback rather than a `RecursionError`.
        """
        root = node
        while self.parent[root] != root:
            root = self.parent[root]
        current = node
        while self.parent[current] != current:
            following = self.parent[current]
            self.parent[current] = root
            current = following
        return root

    def union(self, a: int, b: int) -> None:
        """Merge the sets containing `a` and `b`. Does nothing when they are already one.

        Returns `None` so that calling it and ignoring the outcome is a statement the subset
        accepts. `group_count` is how you ask what happened.
        """
        left = self.find(a)
        right = self.find(b)
        if left == right:
            return
        if self.rank[left] < self.rank[right]:
            held = left
            left = right
            right = held
        self.parent[right] = left
        if self.rank[left] == self.rank[right]:
            self.rank[left] = self.rank[left] + 1
        self.groups = self.groups - 1

    def connected(self, a: int, b: int) -> bool:
        """Whether `a` and `b` are in the same set."""
        return self.find(a) == self.find(b)

    def group_count(self) -> int:
        """How many disjoint sets remain."""
        return self.groups


@c.compyle
def component_count(size: int, edges: list[tuple[int, int]]) -> int:
    """How many connected components `size` nodes fall into, given `edges`.

    Edges are `tuple[int, int]`, so each one is read by position: `edge[0]` and `edge[1]`. The
    position has to be a literal — a tuple is typed per position, so `edge[i]` for a computed `i`
    would have a type that depends on a runtime value, and is rejected.
    """
    sets = UnionFind(size)
    for edge in edges:
        sets.union(edge[0], edge[1])
    return sets.group_count()


@c.compyle
class RunningStats:
    """Mean and variance updated one value at a time, by Welford's algorithm.

    The streaming counterpart to `stats.variance`, and the reason to want a class: it never holds
    the values, so it works over a stream that does not fit in memory — and it is numerically
    better behaved than summing the squares, which loses precision exactly when the mean is large
    next to the spread.
    """

    def __init__(self) -> None:
        self.count: int = 0
        self.average: float = 0.0
        self.sum_of_squares: float = 0.0

    def add(self, value: float) -> None:
        """Fold one more observation in."""
        self.count = self.count + 1
        delta = value - self.average
        self.average = self.average + delta / self.count
        self.sum_of_squares = self.sum_of_squares + delta * (value - self.average)

    def seen(self) -> int:
        """How many observations have been folded in."""
        return self.count

    def mean_value(self) -> float:
        """The mean so far."""
        return self.average

    def variance_value(self) -> float:
        """The population variance so far. Zero until something has been added."""
        if self.count == 0:
            return 0.0
        return self.sum_of_squares / self.count
