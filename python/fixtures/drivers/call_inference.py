"""Calls that exercise `call_inference.py`.

`forward` calls a function defined below it, which is the case the signature pass exists for, and
`promoted` passes an integer where a float is declared -- the conversion is what could be dropped
silently.
"""

CALLS = [
    {"call": "helper", "args": [3]},
    {"call": "f", "args": [4]},
    {"call": "f", "args": [-4]},
    {"call": "forward", "args": [9]},
    {"call": "forward", "args": [-1]},
    {"call": "later", "args": [9]},
    {"call": "promoted", "args": [3]},
    {"call": "promoted", "args": [-3]},
    {"call": "scale_float", "args": [2.5]},
    {"call": "scale_float", "args": [-1.5]},
]
