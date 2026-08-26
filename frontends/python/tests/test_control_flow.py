"""Branches and loops, compared against the interpreted originals.

The Rust tests already run the emitted code, so what these add is the boundary: a recursive
function has to resolve a call to itself through the decorator, where each function is validated on
its own, and a loop's result has to survive the round trip through PyO3.

Recursion is the shape worth checking most: a self-call only resolves because signatures are
gathered before any body is lowered, and a base case that compiled but never fired would loop
forever rather than return something wrong.
"""

from __future__ import annotations

import compylr
import pytest
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]


# Interpreted references. Each is compiled and then compared against the original.
def _factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * _factorial(n - 1)


def _fib(n: int) -> int:
    if n < 2:
        return n
    return _fib(n - 1) + _fib(n - 2)


def _is_prime(n: int) -> bool:
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d = d + 1
    return True


def _nth_prime(n: int) -> int:
    """The iterative counterpart: a counter, a `while`, and a call to another marked function."""
    found = 0
    candidate = 1
    while found < n:
        candidate = candidate + 1
        if _is_prime(candidate):
            found = found + 1
    return candidate


def _sum_to(n: int) -> int:
    total = 0
    for i in range(n):
        total = total + i
    return total


def _countdown(n: int) -> int:
    seen = 0
    for i in range(n, 0, -1):
        seen = seen * 10 + i
    return seen


def _walk(a: int, b: int, c: int) -> int:
    """A range whose step is a runtime value, so a zero step is reachable."""
    seen = 0
    for _i in range(a, b, c):
        seen = seen + 1
    return seen


def _first_over(xs: list[int], limit: int) -> int:
    for x in xs:
        if x > limit:
            return x
    return -1


@pytest.fixture(scope="module")
def project(tmp_path_factory: pytest.TempPathFactory) -> compylr.Manager:
    """A manager with the control-flow functions marked and built once for the whole module."""
    from compylr import _manager

    _manager._reset_for_tests()
    root = tmp_path_factory.mktemp("control_flow") / ".compylr"
    c = compylr.initialize(root=root)

    for function in (
        _factorial,
        _fib,
        _is_prime,
        _nth_prime,
        _sum_to,
        _countdown,
        _walk,
        _first_over,
    ):
        c.compyle(function)
    c.ensure_built()
    return c


class TestRecursion:
    @pytest.mark.parametrize("n", [0, 1, 2, 5, 10])
    def test_factorial_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        assert project._functions["_factorial"](n) == _factorial(n)

    @pytest.mark.parametrize("n", [0, 1, 2, 7, 15])
    def test_fibonacci_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        # Two self-calls in one expression, which is where an off-by-one in the base case would
        # show up as a value rather than as a hang.
        assert project._functions["_fib"](n) == _fib(n)

    def test_the_base_case_actually_fires(self, project: compylr.Manager) -> None:
        # A recursion whose base case never matched would not return at all, so this passing is
        # itself the assertion; the value only confirms which branch ran.
        assert project._functions["_factorial"](1) == 1
        assert project._functions["_fib"](0) == 0


class TestIteration:
    @pytest.mark.parametrize("n", [1, 2, 3, 10, 25])
    def test_nth_prime_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        assert project._functions["_nth_prime"](n) == _nth_prime(n)

    @pytest.mark.parametrize("n", [0, 1, 5, 100])
    def test_a_counting_loop_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        assert project._functions["_sum_to"](n) == _sum_to(n)

    @pytest.mark.parametrize("n", [0, 1, 4])
    def test_a_negative_step_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        # Python's `range(n, 0, -1)` has no Rust equivalent, so this is the one most likely to
        # differ if the loop were emitted as a native range.
        assert project._functions["_countdown"](n) == _countdown(n)

    def test_iterating_a_list_matches_interpreted(self, project: compylr.Manager) -> None:
        compiled = project._functions["_first_over"]
        for xs, limit in ([1, 5, 9], 4), ([1, 2], 100), ([], 0):
            assert compiled(xs, limit) == _first_over(xs, limit)

    def test_a_runtime_step_matches_interpreted(self, project: compylr.Manager) -> None:
        compiled = project._functions["_walk"]
        for a, b, c in (0, 10, 1), (0, 10, 3), (10, 0, -2), (0, 10, -1), (5, 5, 1):
            assert compiled(a, b, c) == _walk(a, b, c)

    def test_a_zero_step_raises_value_error(self, project: compylr.Manager) -> None:
        # Python raises ValueError for `range(a, b, 0)`. The compiled form must agree rather than
        # spin forever, which is the one failure that leaves nothing at all to diagnose from --
        # so this test timing out would be the finding, not the assertion failing.
        with pytest.raises(ValueError):
            _walk(0, 10, 0)
        with pytest.raises(ValueError):
            project._functions["_walk"](0, 10, 0)
