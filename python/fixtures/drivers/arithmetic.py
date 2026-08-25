"""Calls that exercise `arithmetic.py`.

Negative operands throughout: Rust's `/` truncates toward zero and its `%` takes the sign of the
dividend, so a backend that reached for the native operator answers these three wrong and every
positive case right.
"""

CALLS = [
    {"call": "add", "args": [2, 3]},
    {"call": "add", "args": [-2, -3]},
    {"call": "difference", "args": [3, 10]},
    {"call": "product", "args": [-4, 6]},
    {"call": "halve", "args": [7]},
    {"call": "halve", "args": [-7]},
    {"call": "halve", "args": [0]},
    {"call": "modulo", "args": [7, 3]},
    {"call": "modulo", "args": [-7, 3]},
    {"call": "modulo", "args": [7, -3]},
    {"call": "negate", "args": [5]},
    {"call": "negate", "args": [-5]},
    {"call": "negate", "args": [0]},
]
