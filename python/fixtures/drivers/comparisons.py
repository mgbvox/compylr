"""Calls that exercise `comparisons.py`.

Both answers from every predicate, plus the function returning nothing -- a unit return is a
shape the transcript has to render rather than skip.
"""

CALLS = [
    {"call": "is_even", "args": [4]},
    {"call": "is_even", "args": [7]},
    {"call": "is_even", "args": [-4]},
    {"call": "is_even", "args": [-3]},
    {"call": "is_even", "args": [0]},
    {"call": "differs", "args": [1, 2]},
    {"call": "differs", "args": [2, 2]},
    {"call": "at_most", "args": [1, 2]},
    {"call": "at_most", "args": [2, 2]},
    {"call": "at_most", "args": [3, 2]},
    {"call": "greeting", "args": []},
    {"call": "truthy", "args": []},
    {"call": "nothing", "args": []},
]
