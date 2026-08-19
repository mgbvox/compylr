"""Compiled classes through the decorator.

The property this exists for is that state survives a call. It is worth stating against the
contrast the rest of the suite establishes: a collection **argument** is converted by value at the
boundary, so a compiled function cannot mutate its caller's list. An instance is not converted at
all -- the Python object holds the Rust value, and a method borrows it from there -- so a mutated
attribute is exactly what the caller sees next time. That asymmetry is why an attribute can be a
cache while a parameter cannot be mutated.
"""

from __future__ import annotations

import compylr
import pytest
from compylr import _core
from conftest import needs_toolchain

pytestmark = [pytest.mark.slow, needs_toolchain]


class _Counter:
    def __init__(self, start: int) -> None:
        self.count: int = start

    def bump(self, by: int) -> None:
        self.count = self.count + by

    def bump_twice(self, by: int) -> None:
        # Transitive: this mutates only through a call, which is the case a receiver analysis is
        # most likely to get wrong.
        self.bump(by)
        self.bump(by)

    def get(self) -> int:
        return self.count


class _PrimeCache:
    """The demo's third variant: a cache consulted, read, and filled."""

    def __init__(self) -> None:
        self.known: dict[int, bool] = {}
        self.hits: int = 0

    def is_prime(self, n: int) -> bool:
        if n in self.known:
            self.hits = self.hits + 1
            return self.known[n]
        if n < 2:
            self.known[n] = False
            return False
        d = 2
        while d * d <= n:
            if n % d == 0:
                self.known[n] = False
                return False
            d = d + 1
        self.known[n] = True
        return True

    def hit_count(self) -> int:
        return self.hits

    def known_count(self) -> int:
        return len(self.known)


@pytest.fixture(scope="module")
def project(tmp_path_factory: pytest.TempPathFactory) -> compylr.Manager:
    """A manager with both classes marked and built once for the whole module."""
    from compylr import _manager

    _manager._reset_for_tests()
    root = tmp_path_factory.mktemp("classes") / ".compylr"
    c = compylr.initialize(root=root)
    c.compyle(_Counter)
    c.compyle(_PrimeCache)
    c.ensure_built()
    return c


class TestTheTypeIsExposed:
    def test_it_is_constructible_and_its_methods_are_callable(
        self, project: compylr.Manager
    ) -> None:
        counter = project._functions["_Counter"](7)
        assert counter.get() == 7
        counter.bump(3)
        assert counter.get() == 10

    def test_identity_attributes_are_preserved(self, project: compylr.Manager) -> None:
        marked = project._functions["_Counter"]
        assert marked.__name__ == "_Counter"
        assert marked.python_class is _Counter

    def test_wrong_argument_types_raise_type_error(self, project: compylr.Manager) -> None:
        with pytest.raises(TypeError):
            project._functions["_Counter"]("not an int")


class TestStatePersists:
    def test_a_mutation_is_observed_by_a_second_call(self, project: compylr.Manager) -> None:
        # The property the whole change exists for. If a method took a copy of the receiver this
        # would still be 0, and nothing would report an error.
        counter = project._functions["_Counter"](0)
        counter.bump(1)
        counter.bump(1)
        assert counter.get() == 2

    def test_mutation_reaches_through_a_method_call(self, project: compylr.Manager) -> None:
        counter = project._functions["_Counter"](0)
        counter.bump_twice(5)
        assert counter.get() == 10

    def test_two_instances_are_independent(self, project: compylr.Manager) -> None:
        make = project._functions["_Counter"]
        a, b = make(0), make(100)
        a.bump(1)
        assert (a.get(), b.get()) == (1, 100)

    def test_an_instance_the_caller_keeps_holds_its_state(
        self, project: compylr.Manager
    ) -> None:
        held = project._functions["_Counter"](0)
        for _ in range(5):
            held.bump(2)
        assert held.get() == 10


class TestTheMemoizedShape:
    @pytest.mark.parametrize("n", [0, 1, 2, 17, 18, 97])
    def test_it_matches_interpreted(self, project: compylr.Manager, n: int) -> None:
        assert project._functions["_PrimeCache"]().is_prime(n) == _PrimeCache().is_prime(n)

    def test_the_second_call_hits_the_cache(self, project: compylr.Manager) -> None:
        # Not just "the answer is the same" -- the cache must actually be consulted, or the class
        # is doing the work twice and only looks memoized.
        cache = project._functions["_PrimeCache"]()
        assert cache.is_prime(97) is True
        assert cache.hit_count() == 0, "the first call is a miss"
        assert cache.is_prime(97) is True
        assert cache.hit_count() == 1, "the second call must come from the cache"
        assert cache.known_count() == 1, "and must not have added a second entry"


class TestClassesAndFunctionsShareOneBuild:
    def test_both_land_in_the_same_module(self, tmp_path_factory: pytest.TempPathFactory) -> None:
        from compylr import _manager

        _manager._reset_for_tests()
        root = tmp_path_factory.mktemp("shared") / ".compylr"
        c = compylr.initialize(root=root)

        @c.compyle
        def double(n: int) -> int:
            return n * 2

        @c.compyle
        class Box:
            def __init__(self, v: int) -> None:
                self.v: int = v

            def get(self) -> int:
                return self.v

        assert double(21) == 42
        assert Box(3).get() == 3
        assert set(c._sources) == {"double", "Box"}


class TestRejection:
    def test_an_unsupported_class_fails_at_the_decorator(self) -> None:
        c = compylr.initialize()
        with pytest.raises(_core.CompilationError):

            @c.compyle
            class NoInit:
                def get(self) -> int:
                    return 1
