"""Working with words, and the shape of the subset's string support.

Read this before writing string code against compylr, because the boundary is not where anyone
expects: a `str` **cannot be indexed and cannot be iterated**. There is no `s[0]`, no
`for ch in s`, and no `.split()`, `.lower()`, or `.join()` — `append` is the only method in the
subset. What a `str` *can* do is concatenate with `+`, compare, report its `len`, and answer
`in`.

So the unit of work here is the **word**, handed in as a `list[str]` that ordinary interpreted
Python tokenised. That is a real limitation rather than a stylistic choice, and it is the first
thing to fix if you want compylr for text.

Two container behaviours below are worth knowing because they are Python's and are **declared**
rather than inherited: `x in mapping` tests the mapping's **keys**, and `x in some_string` tests
for a **substring**. `len` over text counts **code points** — not UTF-8 bytes as Go's `len`
would, and not UTF-16 units as TypeScript's `.length` would. The three readings agree on ASCII,
which is what would let a wrong one survive a test suite.
"""

from __future__ import annotations

from ._compylr import c


@c.compyle
def word_count(words: list[str]) -> dict[str, int]:
    """How many times each word appears.

    `word in counts` tests the mapping's keys, as Python does. Reading a key that is absent is
    still an error — assignment is what creates one — so the `else` is not optional.
    """
    counts: dict[str, int] = {}
    for word in words:
        if word in counts:
            counts[word] = counts[word] + 1
        else:
            counts[word] = 1
    return counts


@c.compyle
def most_common(words: list[str]) -> str:
    """The most frequent word, ties broken by taking the alphabetically first. `""` when empty.

    The tie-break is not decoration. Iterating a mapping yields its keys in **no guaranteed
    order**, and the order varies between runs — so "whichever tied word came first" would be a
    different answer on different runs of the same program. Any function that iterates a mapping
    and returns one element needs a rule like this one, or it is not a function.
    """
    counts = word_count(words)
    best = ""
    best_count = 0
    for word in counts:
        if counts[word] > best_count:
            best = word
            best_count = counts[word]
        else:
            if counts[word] == best_count:  # noqa: SIM102 - no `and` in the subset
                if word < best:  # noqa: PLR1730 - no `min` in the subset
                    best = word
    return best


@c.compyle
def unique_words(words: list[str]) -> list[str]:
    """Duplicates removed, **first-seen order kept**.

    A list rather than a set, deliberately. A set would express the intent better and would lose
    the order — sets and mappings both iterate in whatever order the underlying map gives. The
    mapping here is used as a seen-set, which is what it is good for: membership, not order.
    """
    seen: dict[str, int] = {}
    out: list[str] = []
    for word in words:
        if word in seen:
            continue
        seen[word] = 1
        out.append(word)
    return out


@c.compyle
def vowel_letters() -> set[str]:
    """The five vowels, as a set.

    A set literal is the only way to build one: there is no `add`, and `set()` is not a call the
    subset resolves. A set is therefore something you receive, test against, or return whole —
    which covers what a lookup table is for, and not much else.
    """
    return {"a", "e", "i", "o", "u"}


@c.compyle
def count_present(words: list[str], wanted: set[str]) -> int:
    """How many of `words` are in `wanted`.

    The set is the point: this is a hash lookup per word rather than a scan of a list, and the
    difference is the whole reason to hand a set across the boundary instead of a list.
    """
    total = 0
    for word in words:
        if word in wanted:
            total = total + 1
    return total


@c.compyle
def total_length(words: list[str]) -> int:
    """The combined length of every word, in **code points**.

    Not bytes. `len("é")` is 1 here and would be 2 under Go's reading of the same operation; the
    IR records which, so the answer does not depend on which backend ran.
    """
    total = 0
    for word in words:
        total = total + len(word)
    return total


@c.compyle
def longest(words: list[str]) -> str:
    """The longest word, the earliest one when several tie. `""` when there are none."""
    best = ""
    best_length = -1
    for word in words:
        if len(word) > best_length:
            best = word
            best_length = len(word)
    return best


@c.compyle
def joined(words: list[str], separator: str) -> str:
    """`separator.join(words)`, written out.

    There are no string methods, so this is the loop `join` would have hidden. It is also
    quadratic — each `+` builds a new string — which `str.join` is not. Compiling something is
    not the same as making it fast, and this is the clearest small example of that in the demo.
    """
    out = ""
    first = True
    for word in words:
        if first:
            out = out + word
            first = False
        else:
            out = out + separator + word
    return out


@c.compyle
def occurrences(haystack: str, needles: list[str]) -> int:
    """How many of `needles` appear anywhere in `haystack`.

    `in` over a string tests for a **substring**, matching Python — and matching Go, C++, and
    TypeScript too, which is why it is one of the three container behaviours the IR deliberately
    does *not* make configurable.
    """
    total = 0
    for needle in needles:
        if needle in haystack:
            total = total + 1
    return total


@c.compyle
def missing(haystack: str, needles: list[str]) -> list[str]:
    """The needles that do not appear in `haystack`, in the order given.

    `not in` is the only negation the subset has — there is no `not` operator — and it is not a
    second form of membership: it lowers to the negation of one, so nothing downstream has to
    remember to honour a flag.
    """
    out: list[str] = []
    for needle in needles:
        if needle not in haystack:
            out.append(needle)
    return out
