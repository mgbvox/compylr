"""Both benchmarks, checked for honesty rather than for speed.

A benchmark that reports a number nobody verified is worse than no benchmark, so what is asserted
here is the *methodology*: that the two modes really are different modes, that they compute the
same answers, and that the timing loop produces a usable number for a workload too fast to time
once.

Deliberately not asserted: that compiled is faster. That depends on the machine and on the
workload — several rows of the breadth benchmark are honestly *slower* compiled — and a suite that
failed when someone's laptop was busy would teach people to ignore it.
"""

from __future__ import annotations

import time
from typing import Any

import pytest

from algorithms import _timing, benchmark
from algorithms.nth_prime import benchmark as prime_benchmark


class TestTheTimingLoop:
    """One implementation, shared by both benchmarks, so it is tested once."""

    def test_a_workload_too_fast_to_time_once_still_measures(self) -> None:
        # A warm cache hit takes hundreds of nanoseconds, and perf_counter's resolution would
        # report that as zero or as one tick. Timing a batch is the difference between a number
        # and a coin flip.
        measured = _timing.per_call(lambda: None, repetitions=2)
        assert 0 < measured < 1e-3, measured

    def test_it_reports_seconds_per_call_not_per_batch(self) -> None:
        # A workload with a known floor: the reported figure must be per call, so scaling the
        # batch must not scale the answer.
        measured = _timing.per_call(lambda: time.sleep(0.001), repetitions=1)
        assert 0.0005 < measured < 0.05, measured

    def test_the_mode_is_asked_of_the_manager_not_of_the_environment(self) -> None:
        # The environment is what the caller requested; the manager is what happened.
        from algorithms._compylr import c

        assert _timing.compylr_enabled() is c.enabled


@pytest.fixture(scope="module")
def prime_runs() -> tuple[dict[str, Any], dict[str, Any]]:
    """One compiled run and one interpreted run of the nth-prime benchmark."""
    n, repetitions = 40, 1
    return (
        prime_benchmark._run_child(n, repetitions, disabled=False),
        prime_benchmark._run_child(n, repetitions, disabled=True),
    )


@pytest.fixture(scope="module")
def algorithm_runs() -> tuple[dict[str, Any], dict[str, Any]]:
    """The same, for the benchmark over every algorithm."""
    return (
        benchmark._run_child(1, 1, disabled=False),
        benchmark._run_child(1, 1, disabled=True),
    )


class TestTheComparisonIsHonest:
    """Asserted for both benchmarks, because the guarantee is the method rather than the numbers."""

    @pytest.fixture(params=["prime", "algorithms"])
    def runs(
        self,
        request: pytest.FixtureRequest,
        prime_runs: tuple[dict[str, Any], dict[str, Any]],
        algorithm_runs: tuple[dict[str, Any], dict[str, Any]],
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        return prime_runs if request.param == "prime" else algorithm_runs

    def test_the_two_runs_really_are_different_modes(
        self, runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        # Without this the benchmark could be comparing a process against itself and reporting a
        # speedup of 1.0 as if it meant something.
        compiled, interpreted = runs
        assert compiled["compiled"] is True
        assert interpreted["compiled"] is False

    def test_both_modes_agree_on_every_answer(
        self, runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        # If they disagreed, every timing would be measuring two different computations.
        compiled, interpreted = runs
        assert compiled["answers"] == interpreted["answers"]

    def test_every_workload_is_measured_in_both_modes(
        self, runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        compiled, interpreted = runs
        assert compiled["seconds"].keys() == interpreted["seconds"].keys()
        for key in compiled["seconds"]:
            assert compiled["seconds"][key] > 0, key
            assert interpreted["seconds"][key] > 0, key

    def test_the_control_row_is_present_in_both(
        self, runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        # The reference is never compiled, so its ratio is what "no difference" looks like on this
        # machine. Reading the other rows against 1.0 instead would overstate every one of them.
        compiled, interpreted = runs
        assert "reference" in compiled["seconds"]
        assert "reference" in interpreted["seconds"]


class TestTheNthPrimeTable:
    def test_it_names_the_noise_floor_and_reports_agreement(
        self, prime_runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        table = prime_benchmark.format_comparison(*prime_runs)
        assert "noise floor" in table
        assert "reference (never compiled)" in table
        assert "Both modes returned the same answer" in table

    def test_the_driver_runs_end_to_end(self, capsys: pytest.CaptureFixture[str]) -> None:
        assert prime_benchmark.main(["--n", "40", "--repetitions", "1"]) == 0
        out = capsys.readouterr().out
        for _, label in prime_benchmark.ROWS:
            assert label in out


class TestTheAlgorithmsTable:
    def test_it_lists_every_workload_and_reports_agreement(
        self, algorithm_runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        table = benchmark.format_comparison(*algorithm_runs)
        for workload in benchmark.workloads(scale=1):
            assert workload.label in table
        assert "noise floor" in table
        assert "Both modes returned the same answer" in table

    def test_the_rows_are_ordered_by_how_much_compiling_helped(
        self, algorithm_runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        # The spread is what the table is for, so it is sorted rather than listed in source order.
        compiled, interpreted = algorithm_runs
        table = benchmark.format_comparison(*algorithm_runs)
        labels = {w.key: w.label for w in benchmark.workloads(scale=1)}
        ratios = sorted(
            ((interpreted["seconds"][k] / compiled["seconds"][k], k) for k in compiled["seconds"]),
            reverse=True,
        )
        positions = [table.index(labels[key]) for _, key in ratios]
        assert positions == sorted(positions)

    def test_the_driver_runs_end_to_end(self, capsys: pytest.CaptureFixture[str]) -> None:
        assert benchmark.main(["--scale", "1", "--repetitions", "1"]) == 0
        assert "noise floor" in capsys.readouterr().out


class TestTheAnswerSignature:
    """Answers cross between processes as strings, and unordered containers must still compare."""

    def test_a_mapping_compares_by_content_rather_than_by_iteration_order(self) -> None:
        # Mapping iteration order is not guaranteed and varies between runs, so comparing `repr`
        # directly would report a disagreement that is really the hash seed.
        assert benchmark._signature({"a": 1, "b": 2}) == benchmark._signature({"b": 2, "a": 1})

    def test_a_set_compares_the_same_way(self) -> None:
        assert benchmark._signature({1, 2, 3}) == benchmark._signature({3, 1, 2})

    def test_different_content_still_differs(self) -> None:
        assert benchmark._signature({"a": 1}) != benchmark._signature({"a": 2})
        assert benchmark._signature([1, 2]) != benchmark._signature([2, 1])

    def test_a_float_is_compared_to_a_sensible_precision(self) -> None:
        # Two processes doing the same arithmetic in the same order agree bit for bit, but that is
        # not a property worth failing a benchmark over.
        assert benchmark._signature(1.0 / 3) == benchmark._signature(0.3333333333333333)
        assert benchmark._signature(1.0) != benchmark._signature(1.001)
