"""Calls that exercise `class_valued_signatures.py`.

The calls cover direct reads, direct and method-driven mutation, compatible forwarding, and newly
owned returned instances whose state remains observable through later method calls.
"""

CALLS = [
    {"new": "Tally", "args": [0], "methods": [["get", []], ["bump", [4]], ["get", []]]},
    {"call": "build", "args": [7], "methods": [["get", []]]},
    {"call": "build", "args": [-1], "methods": [["get", []]]},
    {"call": "read", "args": [{"new": "Tally", "args": [3]}]},
    {"call": "read", "args": [{"new": "Tally", "args": [-3]}]},
    {"call": "mutate", "args": [{"new": "Tally", "args": [3]}, 4]},
    {"call": "mutate_method", "args": [{"new": "Tally", "args": [5]}, 2]},
    {"call": "forward_tally", "args": [{"new": "Tally", "args": [8]}, 3]},
    {
        "call": "build_and_forward",
        "args": [10, 4],
        "methods": [["get", []], ["bump", [2]], ["get", []]],
    },
]
