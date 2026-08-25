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
