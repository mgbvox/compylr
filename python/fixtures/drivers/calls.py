"""Calls that exercise `calls.py`.

`quadruple` calls `double` twice over, nested, which is where a call emitted as a value rather
than as a call would show up.
"""

CALLS = [
    {"call": "double", "args": [5]},
    {"call": "double", "args": [-5]},
    {"call": "double", "args": [0]},
    {"call": "quadruple", "args": [3]},
    {"call": "quadruple", "args": [-2]},
    {"call": "nested_expression", "args": [2, 3]},
    {"call": "nested_expression", "args": [-2, 3]},
]
