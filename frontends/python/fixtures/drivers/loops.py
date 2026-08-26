"""Calls that exercise `loops.py`.

Zero-trip loops, a negative step, a `continue` that must not skip the cursor advance, and an
empty collection. `key_lengths` is given a non-ASCII key because it takes a length inside the
loop, where a byte count would be off by one and still look plausible.
"""

CALLS = [
    {"call": "total", "args": [5]},
    {"call": "total", "args": [0]},
    {"call": "total", "args": [1]},
    {"call": "countdown", "args": [5]},
    {"call": "countdown", "args": [0]},
    {"call": "stepped", "args": [10]},
    {"call": "stepped", "args": [0]},
    {"call": "stepped", "args": [1]},
    {"call": "first_over", "args": [[1, 2, 9], 5]},
    {"call": "first_over", "args": [[1, 2], 5]},
    {"call": "first_over", "args": [[], 5]},
    {"call": "skip_negatives", "args": [[1, -2, 3]]},
    {"call": "skip_negatives", "args": [[-1, -2]]},
    {"call": "skip_negatives", "args": [[]]},
    {"call": "key_lengths", "args": [{"a": 1, "bb": 2}]},
    {"call": "key_lengths", "args": [{"é": 1, "ab": 2}]},
    {"call": "key_lengths", "args": [{}]},
    {"call": "halve_until_odd", "args": [12]},
    {"call": "halve_until_odd", "args": [7]},
    {"call": "halve_until_odd", "args": [0]},
]
