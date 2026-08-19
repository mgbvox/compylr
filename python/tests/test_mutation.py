"""Mutation across the boundary, and the divergence it makes reachable.

The claim worth testing hardest is negative: a caller's list is **not** modified by a compiled
function, because collections cross by value. Lowering rejects mutating a parameter precisely so
that no program can observe this — so what this file checks is that the copy is real, and that the
rejection lands at the decorator rather than at some later call.

The cache-shaped test is the one the memoized demo depends on: membership, read, and insert over a
local mapping, in a loop.
"""

from __future__ import annotations

import compylr
import pytest
from compylr import _core
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]


# Interpreted references. Each is compiled and then compared against the original.
def _evens_below(limit: int) -> list[int]:
    found: list[int] = []
    for n in range(limit):
        if n % 2 == 0:
            found.append(n)
    return found


def _doubled(xs: list[int]) -> list[int]:
    """Builds a local rather than mutating the parameter, which is the workaround."""
    out: list[int] = []
    for x in xs:
        out.append(x * 2)
    return out


def _replace_first(xs: list[int], value: int) -> list[int]:
    copied = xs
    copied[0] = value
    return copied


def _counts(words: list[str]) -> dict[str, int]:
    seen: dict[str, int] = {}
    for word in words:
        if word in seen:
            seen[word] = seen[word] + 1
        else:
            seen[word] = 1
    return seen


def _nth_triangular_memoized(n: int) -> int:
    """A cache consulted, read, and filled — the shape the memoized demo needs."""
    cache: dict[int, int] = {}
    total = 0
    for i in range(n + 1):
        if i in cache:
            total = total + cache[i]
        else:
            cache[i] = i
            total = total + i
    return total


def _has_member(xs: list[int], x: int) -> bool:
    return x in xs


@pytest.fixture(scope="module")
def project(tmp_path_factory: pytest.TempPathFactory) -> compylr.Manager:
    """A manager with the mutating functions marked and built once for the whole module."""
    from compylr import _manager

    _manager._reset_for_tests()
    root = tmp_path_factory.mktemp("mutation") / ".compylr"
    c = compylr.initialize(root=root)

    for function in (
        _evens_below,
        _doubled,
        _replace_first,
        _counts,
        _nth_triangular_memoized,
        _has_member,
    ):
        c.compyle(function)
    c.ensure_built()
    return c


class TestBuildingCollections:
    @pytest.mark.parametrize("limit", [0, 1, 7, 20])
    def test_a_built_sequence_matches_interpreted(
        self, project: compylr.Manager, limit: int
    ) -> None:
        assert project._functions["_evens_below"](limit) == _evens_below(limit)

    def test_a_built_mapping_matches_interpreted(self, project: compylr.Manager) -> None:
        words = ["a", "b", "a", "c", "a", "b"]
        assert project._functions["_counts"](words) == _counts(words)

    def test_membership_matches_interpreted(self, project: compylr.Manager) -> None:
        compiled = project._functions["_has_member"]
        for xs, x in ([1, 2, 3], 2), ([1, 2, 3], 9), ([], 0):
            assert compiled(xs, x) == _has_member(xs, x)

    @pytest.mark.parametrize("n", [0, 1, 5, 30])
    def test_a_cache_shaped_function_matches_interpreted(
        self, project: compylr.Manager, n: int
    ) -> None:
        # Membership, read, and insert over a local mapping, in a loop.
        assert project._functions["_nth_triangular_memoized"](n) == _nth_triangular_memoized(n)


class TestCollectionsCrossByValue:
    def test_the_callers_list_is_unchanged(self, project: compylr.Manager) -> None:
        # The compiled function binds a local to its parameter and writes through the local, which
        # lowering allows because the local is the function's own value. The caller's list is a
        # different object entirely, so it must be untouched.
        original = [1, 2, 3]
        returned = project._functions["_replace_first"](original, 99)
        assert returned == [99, 2, 3], "the function's own copy is modified"
        assert original == [1, 2, 3], (
            "the caller's list must be untouched: collections cross by value"
        )

    def test_the_interpreted_original_does_modify_it(self) -> None:
        # The divergence stated plainly. This is exactly why mutating a *parameter* is rejected:
        # if it compiled, this asymmetry would be a silent wrong answer rather than a rule.
        original = [1, 2, 3]
        _replace_first(original, 99)
        assert original == [99, 2, 3], "Python aliases; compylr copies"

    def test_the_workaround_agrees_with_interpreted(self, project: compylr.Manager) -> None:
        xs = [1, 2, 3]
        assert project._functions["_doubled"](xs) == _doubled(xs)
        assert xs == [1, 2, 3], "building a local leaves the argument alone"


class TestMutatingAParameterIsRejected:
    def test_it_fails_at_the_decorator(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError) as caught:

            @c.compyle
            def appends(xs: list[int]) -> None:
                xs.append(1)

        message = str(caught.value)
        assert "copy" in message, f"the diagnostic must explain the copy, got: {message}"
        assert "caller" in message, f"and name the caller, got: {message}"
