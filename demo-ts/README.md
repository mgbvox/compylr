# compylr TypeScript demo — common algorithms, compiled to Go

A complete TypeScript demonstration project mirroring the Python demo in `./demo/`. Over twenty-five functions and classes covering sorting, arithmetic, floating-point statistics, text processing, graph algorithms, dynamic programming, matrix operations, data structures, and the depth exploration of `nth-prime`.

```bash
cd demo-ts
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

<!-- benchmark:ts-algorithms -->
```
every algorithm, scale=1, per call, best of 5 batches

workload                           compiled    interpreted   speedup
--------------------------------------------------------------------
arithmetic.collatz_length            0.25us         5.29us     21.5x
dynamic.knapsack                     1.36us        15.19us     11.2x
matrices.multiply                   36.02us       306.18us      8.5x
arithmetic.sieve                     4.04us        28.65us      7.1x
stats.standard_deviation             4.37us        14.86us      3.4x
sorting.merge_sort                  11.57us        32.40us      2.8x
sorting.insertion_sort               3.17us         8.25us      2.6x
dynamic.edit_distance                0.51us         1.18us      2.3x
stats.normalize                      4.76us         9.51us      2.0x
graphs.topological_order             4.29us         6.01us      1.4x
reference (never compiled)           1.41us         1.43us      1.0x
matrices.transpose                   7.92us         7.92us      1.0x
text.is_palindrome                   0.11us         0.10us      0.9x
graphs.bfs_distances                 1.22us         0.73us      0.6x
text.word_count                      0.52us         0.21us      0.4x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_scale 1 — measured on Darwin arm64, Node.js 22, 2026-08-25._
<!-- /benchmark:ts-algorithms -->

### The nth prime, three ways

```bash
node --experimental-strip-types src/algorithms/nth_prime/benchmark.ts --n 500
```

<!-- benchmark:ts-nth-prime -->
```
nth prime, n=500, per call, best of 5 batches

workload                           compiled    interpreted   speedup
--------------------------------------------------------------------
recursive                            50.43us       776.80us     15.4x
iterative                            29.76us       376.54us     12.7x
memoized (cold cache)                52.13us       555.09us     10.6x
reference (never compiled)           44.36us        44.08us      1.0x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_n = 500 — measured on Darwin arm64, Node.js 22, 2026-08-25._
<!-- /benchmark:ts-nth-prime -->
