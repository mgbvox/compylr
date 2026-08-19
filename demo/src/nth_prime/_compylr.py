"""The shared manager.

Every variant marks its members against this one manager, so all three compile into a single
extension — which is the arrangement compylr is built around and the thing the demo should show.
"""

from __future__ import annotations

import compylr

c = compylr.initialize()
