"""Calls that exercise `branching.py`.

Every branch of every function, including the boundaries where two arms meet.
"""

CALLS = [
    {"call": "sign", "args": [5]},
    {"call": "sign", "args": [-5]},
    {"call": "sign", "args": [0]},
    {"call": "clamp", "args": [5, 0, 10]},
    {"call": "clamp", "args": [-1, 0, 10]},
    {"call": "clamp", "args": [11, 0, 10]},
    {"call": "clamp", "args": [0, 0, 10]},
    {"call": "describe", "args": [1]},
    {"call": "describe", "args": [100]},
    {"call": "describe", "args": [101]},
]
