"""Calls that exercise `cross_source_callee.py`."""

CALLS = [
    {"call": "remote_helper", "args": [1]},
    {"call": "remote_helper", "args": [0]},
    {"call": "remote_helper", "args": [-1]},
]
