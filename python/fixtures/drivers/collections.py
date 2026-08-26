"""Calls that exercise `collections.py`.

`from_end` indexes from the end, which is a different operation rather than a different number,
and `characters` counts a non-ASCII string -- Python counts characters and Rust's `len` counts
bytes, so an unconverted length answers 6 for a five-character word.
"""

CALLS = [
    {"call": "first_and_count", "args": [[5]]},
    {"call": "first_and_count", "args": [[1, 2, 3]]},
    {"call": "first_and_count", "args": [[-4, 0, 9]]},
    {"call": "lookup", "args": [{"a": 1, "b": 2}, "b"]},
    {"call": "build_list", "args": []},
    {"call": "mapping", "args": []},
    {"call": "unique", "args": []},
    {"call": "pair", "args": []},
    {"call": "nested", "args": [{"k": [7, 8]}, "k"]},
    {"call": "from_end", "args": [[1, 2, 3]]},
    {"call": "from_end", "args": [[9]]},
    {"call": "characters", "args": ["hello"]},
    {"call": "characters", "args": ["héllo"]},
    {"call": "characters", "args": ["日本語"]},
    {"call": "characters", "args": [""]},
]
