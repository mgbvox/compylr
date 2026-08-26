"""Calls that exercise `documented.py`.

A docstring carries no runtime meaning, so these answers must be exactly what the undocumented
forms would give.
"""

CALLS = [
    {"call": "summed", "args": [1, 2]},
    {"call": "summed", "args": [-1, -2]},
    {"call": "described", "args": [4]},
    {"call": "described", "args": [-4]},
    {"call": "undocumented", "args": [0]},
]
