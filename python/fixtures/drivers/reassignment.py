"""Calls that exercise `reassignment.py`.

`accumulate` reassigns the loop counter, which is what makes the loop terminate; `widened`
rebinds a float-declared local to an integer literal.
"""

CALLS = [
    {"call": "increment", "args": [5]},
    {"call": "increment", "args": [-1]},
    {"call": "accumulate", "args": [5]},
    {"call": "accumulate", "args": [0]},
    {"call": "accumulate", "args": [1]},
    {"call": "widened", "args": []},
]
