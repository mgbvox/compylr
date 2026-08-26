"""Generated blocks in Markdown, addressed by markers.

A *region* is a span between `<!-- prefix:name -->` and `<!-- /prefix:name -->`. Whatever sits
between them is output: a generator rewrites it, and editing it by hand is editing output, which
the next run overwrites.

Extracted from `update_benchmarks.py` so a second generator can share it. They are different jobs
with the same output mechanism -- benchmarks measure and take minutes, the subset matrix counts and
takes milliseconds -- and folding the second into the first would make a documentation check depend
on a benchmark run.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


class MarkerError(RuntimeError):
    """A region is missing, duplicated, or malformed.

    Raised rather than silently skipped: a rewrite that quietly writes nothing is how a table goes
    stale while the job that was supposed to keep it fresh reports success.
    """


@dataclass(frozen=True)
class Region:
    """A generated block: which file it lives in, and what produces its contents."""

    name: str
    path: Path
    prefix: str = "benchmark"

    @property
    def opening(self) -> str:
        return f"<!-- {self.prefix}:{self.name} -->"

    @property
    def closing(self) -> str:
        return f"<!-- /{self.prefix}:{self.name} -->"


def find_region(text: str, region: Region) -> tuple[int, int]:
    """Return the character span *between* a region's markers.

    The markers themselves are left in place, so rewriting a region cannot lose it.
    """
    opening, closing = region.opening, region.closing
    if text.count(opening) != 1 or text.count(closing) != 1:
        raise MarkerError(
            f"expected exactly one {opening} and one {closing}; "
            f"found {text.count(opening)} and {text.count(closing)}"
        )
    start = text.index(opening) + len(opening)
    end = text.index(closing)
    if end < start:
        raise MarkerError(f"{closing} appears before {opening}")
    return start, end


def replace_region(text: str, region: Region, body: str) -> str:
    """Return `text` with the named region's contents replaced by `body`."""
    start, end = find_region(text, region)
    return f"{text[:start]}\n{body.strip()}\n{text[end:]}"
