/**
 * Comprehensive benchmark across demo algorithm modules.
 */

import { insertionSort, mergeSort } from './sorting.ts';
import { sieve, power } from './arithmetic.ts';
import { matrixMultiply } from './matrices.ts';
import { editDistance } from './dynamic.ts';
import { runNthPrimeBenchmark } from './nth_prime/benchmark.ts';

export function runAllBenchmarks(): void {
    console.log("=========================================");
    console.log("compylr TypeScript Algorithm Benchmarks");
    console.log("=========================================");

    const time = (name: string, iters: number, fn: () => void) => {
        const start = performance.now();
        for (let i = 0; i < iters; i++) {
            fn();
        }
        const avg = (performance.now() - start) / iters;
        console.log(`  ${name.padEnd(25)}: ${avg.toFixed(4)} ms`);
    };

    console.log("\n1. Sorting & Searching:");
    const testList = Array.from({ length: 500 }, (_, i) => 500 - i);
    time("insertionSort (n=500)", 20, () => insertionSort(testList));
    time("mergeSort (n=500)", 100, () => mergeSort(testList));

    console.log("\n2. Arithmetic:");
    time("sieve(10000)", 50, () => sieve(10000));
    time("power(2, 50)", 1000, () => power(2, 50));

    console.log("\n3. Matrices:");
    const dim = 30;
    const matA = Array.from({ length: dim }, (_, r) => Array.from({ length: dim }, (_, c) => r + c));
    const matB = Array.from({ length: dim }, (_, r) => Array.from({ length: dim }, (_, c) => r * c));
    time(`matrixMultiply (${dim}x${dim})`, 50, () => matrixMultiply(matA, matB));

    console.log("\n4. Dynamic Programming:");
    const wordsA = ["the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog"];
    const wordsB = ["the", "fast", "brown", "fox", "leaps", "over", "a", "sleepy", "dog"];
    time("editDistance (9 words)", 500, () => editDistance(wordsA, wordsB));

    runNthPrimeBenchmark(200, 10);
    console.log("\n=========================================\n");
}

if (import.meta.url === `file://${process.argv[1]}`) {
    runAllBenchmarks();
}
