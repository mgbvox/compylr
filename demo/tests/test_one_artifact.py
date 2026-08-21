"""The whole project compiles into one extension, and is genuinely compiled.

Both halves matter. A demo that quietly fell back to interpreted Python would pass every
correctness test in this directory and demonstrate nothing — so the assertion that the answers
came out of a built extension is the one all the others rest on.

And one artifact is the arrangement compylr is built around rather than an implementation detail:
it is what lets `graphs.node_list` call `sorting.merge_sort`, and it is why a name is unique
across the project rather than within a module.
"""

from __future__ import annotations

import pytest

from algorithms import _compylr, nth_prime, sorting
from algorithms._compylr import c


class TestOneManager:
    def test_the_subpackage_shares_the_package_manager(self) -> None:
        # `initialize` is process-wide: called again with the same settings it hands back the
        # manager that already exists. Two managers would mean two crates, two builds, and
        # compiled functions in one that could not call the other.
        assert nth_prime.c is c
        assert _compylr.c is c

    def test_initializing_again_returns_the_same_manager(self) -> None:
        import compylr

        assert compylr.initialize() is c

    def test_initializing_with_different_settings_is_refused(self) -> None:
        import compylr
        from compylr import ConfigurationError

        # Refused rather than silently re-pointing a project that is already partly configured:
        # the members marked before the change would compile under settings nobody chose.
        with pytest.raises(ConfigurationError):
            compylr.initialize(llm_assist=True)


@pytest.mark.skipif(not c.enabled, reason="compilation is disabled for this process")
class TestOneArtifact:
    def test_every_module_lands_in_the_same_extension(self) -> None:
        module = c.ensure_built()
        assert module.__name__.startswith("compylr_generated_"), (
            f"the answers must come from a compiled extension, not from {module.__name__}"
        )
        for name in (
            "merge_sort",  # sorting
            "sieve",  # arithmetic
            "standard_deviation",  # stats
            "word_count",  # text
            "bfs_distances",  # graphs
            "edit_distance",  # dynamic
            "multiply",  # matrices
            "UnionFind",  # structures
            "recursive_nth_prime",  # nth_prime
            "PrimeCache",  # nth_prime
        ):
            assert hasattr(module, name), f"{name} is missing from the compiled extension"

    def test_building_twice_returns_the_same_module(self) -> None:
        assert c.ensure_built() is c.ensure_built()

    def test_a_cross_module_call_resolves(self) -> None:
        # `graphs.node_list` calls `sorting.merge_sort`, which is only possible because they are
        # in one unit. The annotation on that binding is what lets it be validated separately.
        from algorithms import graphs

        assert graphs.node_list({7: [1], 3: [9]}) == sorting.merge_sort([9, 7, 3, 1])

    def test_names_are_unique_across_the_whole_project(self) -> None:
        # The constraint the flat namespace imposes, asserted rather than assumed: two marked
        # members with one name is a ConfigurationError, and `nth_prime` carries prefixes for
        # exactly this reason.
        marked = list(c._sources)
        assert len(marked) == len(set(marked))
        assert len(marked) > 50, f"the demo should mark the whole subset, found {len(marked)}"
