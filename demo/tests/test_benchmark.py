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
        assert compiled["samples"].keys() == interpreted["samples"].keys()
        for key in compiled["samples"]:
            assert min(compiled["samples"][key]) > 0, key
            assert min(interpreted["samples"][key]) > 0, key

    def test_the_control_row_is_present_in_both(
        self, runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        # The reference is never compiled, so its ratio is what "no difference" looks like on this
        # machine. Reading the other rows against 1.0 instead would overstate every one of them.
        compiled, interpreted = runs
        assert "reference" in compiled["samples"]
        assert "reference" in interpreted["samples"]


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
            (
                (min(interpreted["samples"][k]) / min(compiled["samples"][k]), k)
                for k in compiled["samples"]
            ),
            reverse=True,
        )
        positions = [table.index(labels[key]) for _, key in ratios]
        assert positions == sorted(positions)

    def test_the_driver_runs_end_to_end(self, capsys: pytest.CaptureFixture[str]) -> None:
        assert benchmark.main(["--scale", "1", "--repetitions", "1"]) == 0
        assert "noise floor" in capsys.readouterr().out

    def test_it_reports_interpreted_python_and_rust_behavior_timings(
        self, algorithm_runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        table = benchmark.format_comparison(*algorithm_runs)
        assert "behavior comparison: arithmetic.collatz_length(97)" in table
        assert "interpreted Python" in table
        assert "compiled, Python behavior" in table
        assert "compiled, Rust behavior" in table

    def test_both_behavior_builds_return_the_documented_answer(
        self, algorithm_runs: tuple[dict[str, Any], dict[str, Any]]
    ) -> None:
        compiled, interpreted = algorithm_runs
        assert compiled["answers"]["collatz"] == "118"
        assert compiled["answers"]["collatz_rust"] == "118"
        assert interpreted["answers"]["collatz"] == "118"


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


def _synthetic(
    samples: dict[str, list[float]], *, compiled: bool, scale: int = 1
) -> dict[str, Any]:
    """A Measurement with timings chosen by the caller.

    Built by hand rather than measured, so a test about how the table *reads* a number does not
    depend on what the machine happened to be doing while it ran.
    """
    keys = [workload.key for workload in benchmark.workloads(scale)]
    return {
        "compiled": compiled,
        "scale": scale,
        "repetitions": 5,
        "samples": {key: samples.get(key, [1e-5] * 5) for key in keys},
        "answers": dict.fromkeys(keys, "answer"),
    }


class TestATimingCarriesItsSpread:
    """A figure without an uncertainty is not a measurement, and this repository has been reading
    several as though it were."""

    def test_a_timing_keeps_every_batch_rather_than_only_the_best(self) -> None:
        timing = _timing.measure_call(lambda: None, repetitions=4)
        assert len(timing.samples) == 4
        assert timing.best == min(timing.samples)
        assert timing.worst == max(timing.samples)

    def test_identical_batches_have_no_spread(self) -> None:
        assert _timing.Timing((2e-6, 2e-6, 2e-6)).spread == 0.0

    def test_spread_is_the_range_relative_to_the_best(self) -> None:
        # The observed merge_sort range across byte-identical builds. 73% is wider than most of
        # the improvements anyone would want to measure, which is the whole reason for this work.
        timing = _timing.Timing((160e-6, 202e-6, 277e-6))
        assert timing.spread == pytest.approx((277 - 160) / 160)

    def test_the_best_is_still_available_because_noise_only_adds(self) -> None:
        assert _timing.Timing((3e-6, 1e-6, 2e-6)).best == 1e-6

    def test_per_call_still_reports_a_single_best(self) -> None:
        # Kept, because it is the honest headline figure and every existing caller reads a float.
        measured = _timing.per_call(lambda: None, repetitions=2)
        assert 0 < measured < 1e-3, measured


class TestTheNoiseFloor:
    """Derived from the control row, whose true ratio is 1.0 by construction."""

    def test_a_steady_control_puts_the_floor_at_zero(self) -> None:
        steady = _timing.Timing((1e-5,) * 5)
        assert _timing.noise_floor(steady, steady) == 0.0

    def test_a_control_that_disagrees_with_itself_raises_the_floor(self) -> None:
        # The control is never compiled in either process, so a 1.3x on it is 30% of pure harness.
        compiled = _timing.Timing((1e-5,) * 5)
        interpreted = _timing.Timing((1.3e-5,) * 5)
        assert _timing.noise_floor(compiled, interpreted) == pytest.approx(0.3)

    def test_an_unstable_control_raises_the_floor_even_at_a_ratio_of_one(self) -> None:
        # Both sides can swing wildly while their best-of ratio sits at exactly 1.0. Reading only
        # the ratio would report that run as noiseless.
        jittery = _timing.Timing((1e-5, 1.5e-5))
        assert _timing.noise_floor(jittery, jittery) == pytest.approx(0.5)

    def test_a_difference_inside_the_floor_does_not_resolve(self) -> None:
        assert not _timing.resolves(1.1, floor=0.2)
        assert not _timing.resolves(0.9, floor=0.2)

    def test_a_difference_outside_the_floor_resolves(self) -> None:
        assert _timing.resolves(1.4, floor=0.2)
        assert _timing.resolves(0.5, floor=0.2)

    def test_an_unresolvable_ratio_is_named_rather_than_printed_as_a_result(self) -> None:
        assert _timing.format_ratio(1.05, floor=0.2) == _timing.NOT_RESOLVABLE
        assert _timing.format_ratio(4.1, floor=0.2) == "4.1x"

    def test_a_rows_own_spread_widens_the_floor_it_must_clear(self) -> None:
        # merge_sort moves more run-to-run than the control does, so a 1.2x on it is not a result
        # even on a run whose control was steady.
        wide = _timing.Timing((160e-6, 277e-6))
        floor = _timing.uncertainty(0.05, wide)
        assert floor == pytest.approx((277 - 160) / 160)
        assert not _timing.resolves(1.2, floor)
        # ...but a large enough effect still clears it.
        assert _timing.resolves(8.0, floor)


class TestTheTableReadsAgainstItsOwnNoise:
    """The demo spec's four scenarios, asserted on the printed table."""

    def test_every_timing_is_printed_with_a_spread(self) -> None:
        table = benchmark.format_comparison(
            _synthetic({}, compiled=True), _synthetic({}, compiled=False)
        )
        assert "spread" in table

    def test_the_noise_floor_is_stated(self) -> None:
        table = benchmark.format_comparison(
            _synthetic({}, compiled=True), _synthetic({}, compiled=False)
        )
        assert "noise floor" in table

    def test_a_difference_inside_the_floor_is_not_reported_as_a_ratio(self) -> None:
        # The control disagrees with itself by 20%, so a row 5% apart is the harness moving.
        compiled = _synthetic({"reference": [1e-5] * 5, "sieve": [1e-5] * 5}, compiled=True)
        interpreted = _synthetic(
            {"reference": [1.2e-5] * 5, "sieve": [1.05e-5] * 5}, compiled=False
        )
        table = benchmark.format_comparison(compiled, interpreted)
        row = next(line for line in table.splitlines() if "arithmetic.sieve" in line)
        assert _timing.NOT_RESOLVABLE in row
        assert "1.1x" not in row

    def test_a_difference_outside_the_floor_is_still_reported(self) -> None:
        compiled = _synthetic({"reference": [1e-5] * 5, "sieve": [1e-5] * 5}, compiled=True)
        interpreted = _synthetic({"reference": [1e-5] * 5, "sieve": [4e-5] * 5}, compiled=False)
        table = benchmark.format_comparison(compiled, interpreted)
        row = next(line for line in table.splitlines() if "arithmetic.sieve" in line)
        assert "4.0x" in row

    def test_an_unstable_workload_is_visible_as_unstable(self) -> None:
        # A row that swings 73% run to run must not print a stable-looking number.
        compiled = _synthetic({"merge_sort": [160e-6, 202e-6, 277e-6]}, compiled=True)
        interpreted = _synthetic({"merge_sort": [800e-6] * 3}, compiled=False)
        table = benchmark.format_comparison(compiled, interpreted)
        row = next(line for line in table.splitlines() if "sorting.merge_sort" in line)
        assert _timing.UNSTABLE_MARK in row
        assert "unstable" in table

    def test_a_steady_workload_is_not_marked_unstable(self) -> None:
        compiled = _synthetic({"merge_sort": [160e-6] * 3}, compiled=True)
        interpreted = _synthetic({"merge_sort": [800e-6] * 3}, compiled=False)
        table = benchmark.format_comparison(compiled, interpreted)
        row = next(line for line in table.splitlines() if "sorting.merge_sort" in line)
        assert _timing.UNSTABLE_MARK not in row
