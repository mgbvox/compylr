"""Calls that exercise `division.py`.

The three modes this fixture exists for, each with a negative operand. `//` rounds toward
negative infinity and `%` takes the sign of the divisor; Rust's own operators do neither, and
both disagree only on negatives.
"""

CALLS = [
    {"call": "ratio", "args": [7, 2]},
    {"call": "ratio", "args": [-7, 2]},
    {"call": "ratio", "args": [1, 3]},
    {"call": "ratio", "args": [0, 5]},
    {"call": "halves", "args": [7]},
    {"call": "halves", "args": [-7]},
    {"call": "halves", "args": [0]},
    {"call": "remainder", "args": [7, 3]},
    {"call": "remainder", "args": [-7, 3]},
    {"call": "remainder", "args": [7, -3]},
    {"call": "remainder", "args": [-7, -3]},
]
