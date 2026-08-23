# Implementation notes

## Demo measurement

Measured on 2026-08-23 after `rm -rf .compylr demo/.compylr` and `make demo`, at
`scale=1` with five timing batches:

| mode | best time | spread |
| --- | ---: | ---: |
| interpreted Python | 9.45 µs | 18% |
| compiled, Python behavior | 0.29 µs | 53% |
| compiled, Rust behavior | 0.27 µs | 47% |

The never-compiled control established a **72% noise floor** for this run. The 0.29 µs versus
0.27 µs behavior difference therefore was **not resolvable**; reporting a speedup would claim more
than the harness measured. All three modes returned the documented answer, `118`, for
`collatz_length(97)`.
