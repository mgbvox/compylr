"""Calls that exercise `class_valued_signatures.py`.

`build` bumps the instance it returns, so a mutation dropped on the way out changes an answer,
and `read` receives an instance built as an argument.
"""

CALLS = [
    {"new": "Tally", "args": [0], "methods": [["get", []], ["bump", [4]], ["get", []]]},
    {"call": "build", "args": [7], "methods": [["get", []]]},
    {"call": "build", "args": [-1], "methods": [["get", []]]},
    {"call": "read", "args": [{"new": "Tally", "args": [3]}]},
    {"call": "read", "args": [{"new": "Tally", "args": [-3]}]},
]
