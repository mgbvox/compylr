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
arithmetic.collatz_length               2.70us        58.00us     21.5x
dynamic.knapsack                       18.22us       204.09us     11.2x
matrices.multiply                      64.72us       550.14us      8.5x
arithmetic.sieve                        8.86us        62.94us      7.1x
stats.standard_deviation               18.95us        64.42us      3.4x
sorting.merge_sort                     16.03us        44.88us      2.8x
sorting.insertion_sort                  4.10us        10.67us      2.6x
dynamic.edit_distance                   8.16us        18.76us      2.3x
stats.normalize                        10.57us        21.15us      2.0x
graphs.topological_order                7.36us        10.31us      1.4x
reference (never compiled)              0.50us         0.51us      1.0x
matrices.transpose                     20.78us        20.78us      1.0x
text.is_palindrome                      0.22us         0.20us      0.9x
graphs.bfs_distances                    2.71us         1.63us      0.6x
text.word_count                         0.60us         0.24us      0.4x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_scale 1 — measured on Linux x86_64, Node.js 22, 2026-10-26._
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
recursive                              62.22us      1081.34us     17.4x
iterative                              44.02us       564.55us     12.8x
memoized (cold cache)                  57.62us       634.59us     11.0x
reference (never compiled)             61.00us        61.52us      1.0x

The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.
Both modes returned the same answer for every workload.
```

_n = 500 — measured on Linux x86_64, Node.js 22, 2026-10-26._
<!-- /benchmark:nth-prime -->
