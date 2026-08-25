# Implementation notes

## Demo measurement

Measured on 2026-08-23 after `rm -rf .compylr demo/.compylr` and `make demo`, at `scale=1` with
five timing batches:

| mode | best time | spread |
| --- | ---: | ---: |
| interpreted Python | 6.10 µs | 0% |
| compiled, Python behavior | 0.27 µs | 1% |
| compiled, Rust behavior | 0.25 µs | 2% |

The never-compiled control established a **6% noise floor** for this run, which the ~8-12% gap
between the two compiled builds clears, so the harness reported **1.1x** rather than declining to.
All three modes returned the documented answer, `118`, for `collatz_length(97)`.

### The behavior comparison sits close to the floor, and that is the finding

That 1.1x is not stable enough to quote on its own. Across **eight** runs against the same build on
an otherwise idle machine, the compiled timings barely moved — Python behavior 0.27-0.28 µs, Rust
behavior 0.25 µs on every single run — but the floor moved between 2% and 12%, and **one run in
eight reported `not resolvable`**: the one whose Python-behavior row carried a 12% spread of its
own, which `uncertainty` correctly folds in.

So the honest statement is that the Rust stance is worth roughly a tenth of the compiled time for
this loop, and that this is near the limit of what the harness can see. It is not a headline.

An earlier recording of this same comparison, on a loaded machine, read 9.45 µs / 0.29 µs / 0.27 µs
at a **72% noise floor** and reported the difference as **not resolvable**. That was the correct
answer for that run. Keeping both recordings is the point: the same small gap is reportable at a
6% floor and is not at 72%, and task 11.4's discipline is what makes the difference visible instead
of letting one run's number become a claim.

The claim that survives every floor the harness has produced is the one against interpretation:
better than twenty times, on every run.
