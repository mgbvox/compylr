"""Calls that exercise `cross_source_caller.py`.

`caller` reaches a function defined in another source, so this fixture only means anything when
its pair is present -- both tiers group the two the way `emit_quality.rs` already does.
"""

CALLS = [
    {"call": "caller", "args": [5]},
    {"call": "caller", "args": [0]},
    {"call": "caller", "args": [-5]},
]
