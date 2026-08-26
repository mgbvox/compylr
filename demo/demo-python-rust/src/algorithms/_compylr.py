"""The manager this package marks its members against.

Deliberately a second call to `compylr.initialize()` rather than an import of `nth_prime`'s
manager. `initialize` is process-wide: called again with the same settings it hands back the
manager that already exists, which is what keeps every decorated member in a project — across
packages that know nothing about each other — inside **one** shared extension.

`tests/test_one_artifact.py` asserts that identity rather than trusting it. Two managers would
mean two crates, two builds, and compiled functions in one that could not call the other.
"""

from __future__ import annotations

import compylr

c = compylr.initialize()
