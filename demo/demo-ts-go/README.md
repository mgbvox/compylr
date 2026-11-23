# compylr TypeScript demo — common algorithms, compiled to Go

A complete TypeScript demonstration project mirroring the Python demo in
[`../demo-python-rust/`](../demo-python-rust/). Over twenty-five functions and classes covering
sorting, arithmetic, floating-point statistics, text processing, graph algorithms, dynamic
programming, matrix operations, data structures, and the depth exploration of `nth-prime`.

The two demos carry the same algorithms on purpose. compylr's claim is that a program's meaning
survives the trip through an IR that names neither the language it came from nor the one it
becomes, and two pipelines doing identical work is how that claim is put where it can be checked
rather than asserted.

```bash
cd demo/demo-ts-go
npm test        # run the complete test suite against reference oracles
npm start       # display IR construct coverage and run algorithm benchmarks
npm run benchmark # run benchmark suites across all algorithm domains
```

## The two halves

The demo provides full architectural and algorithmic parity with the Python demo:

### 1. Breadth — [`src/algorithms/`](src/algorithms/)

| Module | What it covers |
| --- | --- |
| [`sorting.ts`](src/algorithms/sorting.ts) | Insertion sort, selection sort, merge sort, binary search, sortedness check |
| [`arithmetic.ts`](src/algorithms/arithmetic.ts) | GCD, LCM, integer square root, binary exponentiation, Collatz sequence, Eratosthenes sieve, digit sum, division and base conversion |
| [`stats.ts`](src/algorithms/stats.ts) | Mean, count averages, variance, Newton's square root, standard deviation, extremes, normalization, and `RunningStats` accumulator class |
| [`text.ts`](src/algorithms/text.ts) | Word frequency mapping, most common word resolution, unique word tracking, vowel lookup set, subset membership, and palindrome checking |
| [`graphs.ts`](src/algorithms/graphs.ts) | Node list collection, BFS shortest hop distances, DFS traversal order, Kahn's topological sort, and connected component counting |
| [`dynamic.ts`](src/algorithms/dynamic.ts) | Levenshtein edit distance, longest common subsequence (LCS), coin change, 0/1 knapsack, and Kadane's maximum subarray sum |
| [`matrices.ts`](src/algorithms/matrices.ts) | Matrix multiplication, transposition, and matrix trace calculation |
| [`structures.ts`](src/algorithms/structures.ts) | `IntStack` (bracket balance validator) and `UnionFind` with path compression and rank optimization |

### 2. Depth — [`src/algorithms/nth_prime/`](src/algorithms/nth_prime/)

One problem, three implementation variants, asserted to agree with each other and with an interpreted oracle:

| Variant | Exercises |
| --- | --- |
| [`recursive.ts`](src/algorithms/nth_prime/recursive.ts) | Recursion over prime counts with base cases, branching, and cross-function calls |
| [`iterative.ts`](src/algorithms/nth_prime/iterative.ts) | `while` loops, prime table generation, `break`, and local collection building |
| [`memoized.ts`](src/algorithms/nth_prime/memoized.ts) | `PrimeCache` class with a persistent cache map and hit counting |
| [`reference.ts`](src/algorithms/nth_prime/reference.ts) | Uncompiled pure reference oracle for differential verification |

## Verification & Testing

Every algorithm is verified against reference oracles and properties in [`tests/`](tests/):
- **`tests/test_algorithms.test.ts`**: Verifies all breadth algorithms against known values, mathematical properties, and edge cases.
- **`tests/test_nth_prime.test.ts`**: Verifies agreement across recursive, iterative, memoized, and reference implementations.
- **`tests/test_coverage.test.ts`**: Asserts all demo functions and classes are registered and accounted for.

## Benchmarks

Timings are the best of several batches per call, comparing compiled Go execution to interpreted TypeScript.

### Every algorithm

```bash
npm run benchmark
```

<!-- benchmark:algorithms -->
```
every algorithm, scale=1, per call, best of 5 batches

workload                           compiled    interpreted   speedup
--------------------------------------------------------------------
arithmetic.collatz_length               4.66us       100.23us     21.5x
dynamic.knapsack                       27.70us       310.23us     11.2x
matrices.multiply                     123.49us      1049.67us      8.5x
arithmetic.sieve                        9.95us        70.62us      7.1x
stats.standard_deviation               27.91us        94.88us      3.4x
sorting.merge_sort                     29.92us        83.79us      2.8x
sorting.insertion_sort                  8.03us        20.87us      2.6x
dynamic.edit_distance                  13.62us        31.33us      2.3x
stats.normalize                        20.90us        41.80us      2.0x
graphs.topological_order               12.75us        17.85us      1.4x
reference (never compiled)              0.96us         0.95us      1.0x
matrices.transpose                     21.45us        21.45us      1.0x
text.is_palindrome                      0.22us         0.20us      0.9x
graphs.bfs_distances                    2.84us         1.70us      0.6x
text.word_count                         0.62us         0.25us      0.4x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_scale 1 — measured on Linux x86_64, Node.js 22, 2026-11-23._
<!-- /benchmark:algorithms -->

### The nth prime, three ways

```bash
node --experimental-strip-types src/algorithms/nth_prime/benchmark.ts --n 500
```

<!-- benchmark:nth-prime -->
```
nth prime, n=500, per call, best of 5 batches

workload                           compiled    interpreted   speedup
--------------------------------------------------------------------
recursive                              66.79us       994.58us     14.9x
iterative                              46.70us       630.29us     13.5x
memoized (cold cache)                  60.11us       694.22us     11.5x
reference (never compiled)             58.87us        57.71us      1.0x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_n = 500 — measured on Linux x86_64, Node.js 22, 2026-11-23._
<!-- /benchmark:nth-prime -->
