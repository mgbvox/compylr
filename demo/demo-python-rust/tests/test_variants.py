"""Every variant, against the interpreted oracle and against each other.

Three implementations that compile but disagree is the failure worth catching, so agreement is
asserted directly rather than inferred from each one matching the reference.

The build is shared across the module: each variant compiles into the same extension, so paying for
it once is both faster and a check that one build really does cover all three.
"""

from __future__ import annotations

import pytest

from algorithms.nth_prime import iterative, memoized, recursive, reference

#: The range every variant is checked over.
#:
#: Bounded because the recursive variant recurses once per prime found and there is no tail-call
#: elimination. Measured on this machine: n=100,000 succeeds, n=150,000 aborts the process with
#: SIGSEGV and no traceback. The tests stay far below that; the README states the bound.
CHECKED = list(range(1, 60))

FIRST_FIVE = [2, 3, 5, 7, 11]


@pytest.fixture(scope="module")
def cache() -> memoized.PrimeCache:
    return memoized.PrimeCache()


class TestTheReference:
    def test_it_returns_the_first_five_primes(self) -> None:
        # The oracle is checked against known values rather than against the variants, or every
        # test in this file would agree on whatever the reference happened to do.
        assert [reference.nth_prime(n) for n in range(1, 6)] == FIRST_FIVE


class TestEachVariantMatchesTheReference:
    @pytest.mark.parametrize("n", CHECKED)
    def test_recursive(self, n: int) -> None:
        assert recursive.nth_prime(n) == reference.nth_prime(n)

    @pytest.mark.parametrize("n", CHECKED)
    def test_iterative(self, n: int) -> None:
        assert iterative.nth_prime(n) == reference.nth_prime(n)

    @pytest.mark.parametrize("n", CHECKED)
    def test_memoized(self, n: int, cache: memoized.PrimeCache) -> None:
        assert cache.nth(n) == reference.nth_prime(n)


class TestTheVariantsAgree:
    @pytest.mark.parametrize("n", CHECKED)
    def test_all_three_agree(self, n: int, cache: memoized.PrimeCache) -> None:
        answers = {recursive.nth_prime(n), iterative.nth_prime(n), cache.nth(n)}
        assert len(answers) == 1, f"the variants disagreed for n={n}: {answers}"

    @pytest.mark.parametrize("n", [0, -1, -100])
    def test_the_edge_below_one_is_defined(self, n: int, cache: memoized.PrimeCache) -> None:
        # Defined rather than discovered: the compiled subset has no exceptions of its own, so
        # every variant returns 0 and says so in its docstring.
        assert recursive.nth_prime(n) == 0
        assert iterative.nth_prime(n) == 0
        assert cache.nth(n) == 0
        assert reference.nth_prime(n) == 0


class TestTheCacheIsUsed:
    def test_a_repeat_request_increments_the_hit_counter(self) -> None:
        # Without this, "memoized" is a claim about the code's shape. A refactor that broke caching
        # would still pass every agreement test above.
        fresh = memoized.PrimeCache()
        assert fresh.hit_count() == 0
        fresh.nth(50)
        assert fresh.hit_count() == 0, "the first request is a miss"
        fresh.nth(50)
        assert fresh.hit_count() == 1, "the second must come from the cache"
        assert fresh.known_count() == 1, "and must not have stored a second answer"

    def test_two_instances_hold_independent_caches(self) -> None:
        a, b = memoized.PrimeCache(), memoized.PrimeCache()
        a.nth(10)
        a.nth(10)
        assert a.hit_count() == 1
        assert b.hit_count() == 0, "one instance's cache must not serve another"


class TestTheyAreGenuinelyCompiled:
    def test_no_variant_silently_falls_back(self) -> None:
        # A demo that quietly ran interpreted Python would demonstrate nothing at all, and every
        # assertion above would still pass.
        from algorithms._compylr import c

        module = c.ensure_built()
        for name in (
            "recursive_nth_prime",
            "iterative_nth_prime",
            "PrimeCache",
        ):
            assert hasattr(module, name), f"{name} is missing from the compiled extension"

    def test_one_build_covers_all_three(self) -> None:
        from algorithms._compylr import c

        marked = set(c._sources)
        assert {"recursive_nth_prime", "iterative_nth_prime", "PrimeCache"} <= marked
        assert c.ensure_built() is c.ensure_built(), "all three share one artifact"
