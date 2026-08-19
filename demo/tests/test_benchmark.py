"""The benchmark, checked for honesty rather than for speed.

A benchmark that reports a number nobody verified is worse than no benchmark, so what is asserted
here is the *methodology*: that the two modes really are different modes, that they compute the same
answers, and that the timing loop produces a usable number for a workload too fast to time once.

Deliberately not asserted: that compiled is faster. That depends on the machine, and a suite that
fails when someone's laptop is busy teaches people to ignore it.
"""

from __future__ import annotations

import pytest

from nth_prime import benchmark


class TestTheTimingLoop:
    def test_a_workload_too_fast_to_time_once_still_measures(self) -> None:
        # A warm cache hit takes hundreds of nanoseconds, and perf_counter's resolution would
        # report that as zero or as one tick. Timing a batch is the difference between a number
        # and a coin flip.
        per_call = benchmark._per_call(lambda: None, repetitions=2)
        assert 0 < per_call < 1e-3, per_call

    def test_it_reports_seconds_per_call_not_per_batch(self) -> None:
        import time

        # A workload with a known floor: the reported figure must be per call, so scaling the
        # batch must not scale the answer.
        measured = benchmark._per_call(lambda: time.sleep(0.001), repetitions=1)
        assert 0.0005 < measured < 0.05, measured


@pytest.fixture(scope="module")
def both() -> tuple[dict[str, object], dict[str, object]]:
    """One compiled run and one interpreted run, shared across the assertions about them."""
    n, reps = 40, 1
    return (
        benchmark._run_child(n, reps, disabled=False),
        benchmark._run_child(n, reps, disabled=True),
    )


class TestTheComparisonIsHonest:

    def test_the_two_runs_really_are_different_modes(
        self, both: tuple[dict[str, object], dict[str, object]]
    ) -> None:
        # Without this the benchmark could be comparing a process against itself and reporting a
        # speedup of 1.0 as if it meant something.
        compiled, interpreted = both
        assert compiled["compiled"] is True
        assert interpreted["compiled"] is False

    def test_both_modes_agree_on_every_answer(
        self, both: tuple[dict[str, object], dict[str, object]]
    ) -> None:
        # If they disagreed, every timing would be measuring two different computations.
        compiled, interpreted = both
        assert compiled["answers"] == interpreted["answers"]

    def test_every_row_is_measured_in_both_modes(
        self, both: tuple[dict[str, object], dict[str, object]]
    ) -> None:
        compiled, interpreted = both
        for key, _ in benchmark.ROWS:
            assert compiled["seconds"][key] > 0, key
            assert interpreted["seconds"][key] > 0, key

    def test_the_table_reports_the_control_row_as_the_noise_floor(
        self, both: tuple[dict[str, object], dict[str, object]]
    ) -> None:
        # The reference is never compiled, so its ratio is what "no difference" looks like on this
        # machine. Reading the other rows against 1.0 instead would overstate every one of them.
        table = benchmark.format_comparison(*both)
        assert "noise floor" in table
        assert "reference (never compiled)" in table
        assert "Both modes returned the same answer" in table


def test_the_driver_runs_end_to_end(capsys: pytest.CaptureFixture[str]) -> None:
    assert benchmark.main(["--n", "40", "--repetitions", "1"]) == 0
    out = capsys.readouterr().out
    for _, label in benchmark.ROWS:
        assert label in out
