"""Calls that exercise `floats.py`.

`scale(0.1, 0.2)` is the one that reads oddly and is right: binary floating point answers
0.020000000000000004, and a transcript that rounded it away would hide a real difference.
"""

CALLS = [
    {"call": "scale", "args": [2.5, 4.0]},
    {"call": "scale", "args": [-1.5, 2.0]},
    {"call": "scale", "args": [0.1, 0.2]},
    {"call": "widen", "args": [3]},
    {"call": "widen", "args": [-3]},
    {"call": "widen", "args": [0]},
    {"call": "mixed", "args": [2, 0.5]},
    {"call": "mixed", "args": [-2, 0.25]},
    {"call": "compare_mixed", "args": [1, 1.5]},
    {"call": "compare_mixed", "args": [2, 1.5]},
]
