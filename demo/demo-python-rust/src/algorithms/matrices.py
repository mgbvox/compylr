"""Dense integer matrices, as lists of rows.

The triple-nested loop in `multiply` is the demo's best case for compiling: it is arithmetic and
indexing all the way down, with nothing for the interpreter to do but dispatch. It is also the
clearest place to see what crossing the boundary costs — at small sizes the conversion of the
argument dominates and compiling loses, and the benchmark reports both sizes rather than only the
flattering one.

`table_of_zeros` comes from `dynamic.py`. A call to a function in another module has to be
annotated where its result is bound, because at the moment this function is validated that
signature is not yet visible.
"""

from __future__ import annotations

from ._compylr import c
from .dynamic import table_of_zeros


@c.compyle
def identity(size: int) -> list[list[int]]:
    """The `size` by `size` identity matrix."""
    out: list[list[int]] = table_of_zeros(size, size)
    for i in range(size):
        out[i][i] = 1
    return out


@c.compyle
def transpose(matrix: list[list[int]]) -> list[list[int]]:
    """`matrix` with rows and columns exchanged. Empty for an empty matrix."""
    rows = len(matrix)
    if rows == 0:
        return []
    columns = len(matrix[0])
    out: list[list[int]] = table_of_zeros(columns, rows)
    for i in range(rows):
        for j in range(columns):
            out[j][i] = matrix[i][j]
    return out


@c.compyle
def multiply(left: list[list[int]], right: list[list[int]]) -> list[list[int]]:
    """The matrix product. Empty when the shapes do not line up.

    Empty rather than an exception, as everywhere else in this demo — and checked rather than
    assumed, because indexing past the end of a row is a panic in the generated code, which
    reaches Python as an exception but not one that says anything useful about matrices.
    """
    rows = len(left)
    if rows == 0:
        return []
    inner = len(left[0])
    if len(right) != inner:
        return []
    if inner == 0:
        return []
    columns = len(right[0])
    out: list[list[int]] = table_of_zeros(rows, columns)
    for i in range(rows):
        for j in range(columns):
            total = 0
            for k in range(inner):
                total = total + left[i][k] * right[k][j]
            out[i][j] = total
    return out


@c.compyle
def trace(matrix: list[list[int]]) -> int:
    """The sum along the leading diagonal. Zero for an empty matrix."""
    rows = len(matrix)
    if rows == 0:
        return 0
    columns = len(matrix[0])
    total = 0
    limit = rows
    if columns < limit:  # noqa: PLR1730 - no `min` in the subset
        limit = columns
    for i in range(limit):
        total = total + matrix[i][i]
    return total


@c.compyle
def row_sums(matrix: list[list[int]]) -> list[int]:
    """The total of each row.

    The `for row in matrix` loop reads each row once. A `for` **snapshots what it iterates**, so
    rebinding `matrix` in the body could not change what is walked — which is what Python's `for`
    does too, and is why the emitted loop clones rather than holding a borrow across the body.
    """
    out: list[int] = []
    for row in matrix:
        total = 0
        for value in row:
            total = total + value
        out.append(total)
    return out


@c.compyle
def scale(matrix: list[list[int]], factor: int) -> list[list[int]]:
    """Every element multiplied by `factor`.

    Builds a fresh matrix rather than writing into the argument. It could not do otherwise: a
    collection parameter crosses the boundary by value, so a mutation here would be invisible to
    the caller — and compylr rejects mutating a parameter for exactly that reason rather than
    compiling a program whose two versions disagree.
    """
    out: list[list[int]] = []
    for row in matrix:
        scaled: list[int] = []
        for value in row:
            scaled.append(value * factor)
        out.append(scaled)
    return out
