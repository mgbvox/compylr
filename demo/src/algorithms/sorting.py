"""Sorting and searching.

The four algorithms here are the ones everybody has written, which is the point: they are the
fastest way to see what the subset feels like to program in. Two constraints shape all of them.

**There is no `pop`, no `insert`, and no swap.** `append` is the only collection method, and
`a, b = b, a` is not in the subset, so an exchange is spelled with a temporary. That is more
verbose and no less clear.

**A collection parameter is a copy and may not be mutated.** An in-place sort is therefore not
expressible — not because sorting in place is hard, but because the caller could never see the
result. Every function here builds a fresh list and returns it, which is the shape the rule
pushes you toward and the one that was probably wanted anyway.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def copy_of(xs: list[int]) -> list[int]:
    """A fresh list with the same elements.

    The first line of every sort here. `out = xs` would bind a second name to the same list in
    Python and copy in compylr, so mutating it is the same hazard one line further out —
    compylr rejects that transitively rather than letting the two languages disagree silently.
    """
    out: list[int] = []
    for x in xs:
        out.append(x)  # noqa: PERF402 - no `list.copy` in the subset
    return out


@c.compyle
def insertion_sort(xs: list[int]) -> list[int]:
    """`xs` in ascending order, by insertion sort.

    O(n^2), and the one worth reading: the inner loop is where `and` would normally go.
    `while j >= 0 and out[j] > key` has no spelling here, so the second half of the condition
    becomes an `if` with a `break`. Same loop, one line longer.
    """
    out = copy_of(xs)
    i = 1
    while i < len(out):
        key = out[i]
        j = i - 1
        while j >= 0:
            if out[j] > key:
                out[j + 1] = out[j]
                j = j - 1
            else:
                break
        out[j + 1] = key
        i = i + 1
    return out


@c.compyle
def selection_sort(xs: list[int]) -> list[int]:
    """`xs` in ascending order, by repeatedly selecting the smallest remaining element.

    Included beside insertion sort because it is where the missing swap shows: exchanging two
    elements takes a temporary, and forgetting it is a bug the compiler cannot catch.
    """
    out = copy_of(xs)
    i = 0
    while i < len(out):
        smallest = i
        j = i + 1
        while j < len(out):
            if out[j] < out[smallest]:
                smallest = j
            j = j + 1
        held = out[i]
        out[i] = out[smallest]
        out[smallest] = held
        i = i + 1
    return out


@c.compyle
def merge(left: list[int], right: list[int]) -> list[int]:
    """Two ascending lists interleaved into one.

    Stable: the `<=` is what keeps equal elements in the order they arrived, and turning it into
    `<` would silently make merge sort unstable.
    """
    out: list[int] = []
    i = 0
    j = 0
    while i < len(left):
        if j >= len(right):
            break
        if left[i] <= right[j]:
            out.append(left[i])
            i = i + 1
        else:
            out.append(right[j])
            j = j + 1
    while i < len(left):
        out.append(left[i])
        i = i + 1
    while j < len(right):
        out.append(right[j])
        j = j + 1
    return out


@c.compyle
def merge_sort(xs: list[int]) -> list[int]:
    """`xs` in ascending order, by merge sort. O(n log n), and stable.

    The halves are built by a loop rather than by `xs[:mid]`, because slicing is not in the
    subset. Worth knowing what that costs: each recursive call takes its argument **by value**,
    so this copies at every level. So does the interpreted version, which builds two new lists
    per call — the difference is that here it is a `memcpy` of a `Vec<i64>` rather than a list
    of boxed integers.
    """
    if len(xs) <= 1:
        return copy_of(xs)
    middle = len(xs) // 2
    left: list[int] = []
    right: list[int] = []
    index = 0
    for x in xs:
        if index < middle:
            left.append(x)
        else:
            right.append(x)
        index = index + 1
    return merge(merge_sort(left), merge_sort(right))


@c.compyle
def is_sorted(xs: list[int]) -> bool:
    """Whether `xs` is in non-descending order.

    The oracle every sort here is checked against, expressed in the subset so the check itself
    is compiled too.
    """
    i = 1
    while i < len(xs):
        if xs[i - 1] > xs[i]:
            return False
        i = i + 1
    return True


@c.compyle
def binary_search(xs: list[int], target: int) -> int:
    """The index of `target` in the ascending `xs`, or -1 when it is absent.

    -1 rather than an exception: the compiled subset has no exceptions of its own, so a sentinel
    is the only answer available. It is documented rather than discovered, and `-1` is also a
    valid index into a list, which is why the caller must treat it as "absent" and not index with
    it — `xs[-1]` counts from the end here exactly as it does in Python.
    """
    low = 0
    high = len(xs) - 1
    found = -1
    while low <= high:
        middle = (low + high) // 2
        if xs[middle] == target:
            found = middle
            break
        if xs[middle] < target:
            low = middle + 1
        else:
            high = middle - 1
    return found
