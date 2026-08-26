"""Calls that exercise `mutation.py`.

Membership over all four containers, including a non-ASCII substring, and empty collections
wherever one is legal. `counts` is the one where a mutation applied to a copy would be silently
lost and every answer still plausible.
"""

CALLS = [
    {"call": "evens_below", "args": [10]},
    {"call": "evens_below", "args": [1]},
    {"call": "evens_below", "args": [0]},
    {"call": "replace_first", "args": [9]},
    {"call": "replace_first", "args": [-9]},
    {"call": "counts", "args": [["a", "b", "a"]]},
    {"call": "counts", "args": [[]]},
    {"call": "has_element", "args": [[1, 2], 2]},
    {"call": "has_element", "args": [[1, 2], 3]},
    {"call": "has_element", "args": [[], 1]},
    {"call": "has_key", "args": [{"a": 1}, "a"]},
    {"call": "has_key", "args": [{"a": 1}, "z"]},
    {"call": "has_key", "args": [{}, "a"]},
    {"call": "has_member", "args": [{1, 2}, 2]},
    {"call": "has_member", "args": [{1, 2}, 3]},
    {"call": "has_substring", "args": ["héllo", "é"]},
    {"call": "has_substring", "args": ["hello", "ell"]},
    {"call": "has_substring", "args": ["hello", "z"]},
    {"call": "has_substring", "args": ["hello", ""]},
    {"call": "lacks_element", "args": [[1, 2], 3]},
    {"call": "lacks_element", "args": [[1, 2], 1]},
]
