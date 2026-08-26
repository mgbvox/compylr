"""Calls that exercise `nested_mutation.py`.

Every write here goes through a nested place. Each one was a live defect once: the write landed
on a copy, the answer stayed plausible, and every correctness test passed.
"""

CALLS = [
    {"call": "zeros", "args": [2, 3]},
    {"call": "zeros", "args": [0, 0]},
    {"call": "zeros", "args": [1, 0]},
    {"call": "diagonal", "args": [3]},
    {"call": "diagonal", "args": [1]},
    {"call": "bucket", "args": ["k", 5]},
    {"call": "bucket", "args": ["k", -5]},
    {
        "new": "Grid",
        "args": [3],
        "methods": [
            ["read", [0, 0]],
            ["write", [1, 2, 7]],
            ["read", [1, 2]],
            ["read", [0, 0]],
            ["write", [0, 0, -1]],
            ["read", [0, 0]],
        ],
    },
]
