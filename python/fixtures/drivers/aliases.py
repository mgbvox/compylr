"""Calls that exercise `aliases.py`.

An alias must carry the value, not a copy of the name, so each call also runs a value the
identity function would hide -- zero, and both booleans.
"""

CALLS = [
    {"call": "alias_parameter", "args": [-5]},
    {"call": "alias_parameter", "args": [0]},
    {"call": "alias_local", "args": []},
    {"call": "alias_chain", "args": [True]},
    {"call": "alias_chain", "args": [False]},
    {"call": "annotated_alias", "args": [42]},
]
