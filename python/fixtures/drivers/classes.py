"""Calls that exercise `classes.py`.

An instance is observed only through its methods -- a generated `#[pyclass]` exposes no field --
so each class is constructed and then read, with the reads interleaved so that a mutation lost
between calls changes an answer. That is the case that matters: an attribute is what outlives a
call, and a mutated copy would be plausible and wrong.
"""

CALLS = [
    {
        "new": "Counter",
        "args": [0],
        "methods": [
            ["get", []],
            ["bump", [5]],
            ["get", []],
            ["bump_twice", [2]],
            ["get", []],
            ["bump", [-3]],
            ["get", []],
        ],
    },
    {
        "new": "Cache",
        "args": [],
        "methods": [
            ["size", []],
            ["has", [1]],
            ["put", [1, 10]],
            ["has", [1]],
            ["size", []],
            ["put", [2, 20]],
            ["put", [1, 11]],
            ["size", []],
            ["has", [3]],
        ],
    },
]
