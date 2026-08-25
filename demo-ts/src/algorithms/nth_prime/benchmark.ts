/**
 * Benchmark comparing interpreted vs compiled nth-prime.
 */

import { recursiveNthPrime } from './recursive.ts';
import { iterativeNthPrime } from './iterative.ts';
import { PrimeCache } from './memoized.ts';
import { referenceNthPrime } from './reference.ts';

export function runNthPrimeBenchmark(n: number = 200, iterations: number = 10): void {
    console.log(`\nBenchmarking nth-prime(n=${n}, iterations=${iterations}):\n`);

    const runTimed = (name: string, fn: () => number) => {
        const start = performance.now();
        let lastResult = 0;
        for (let i = 0; i < iterations; i++) {
            lastResult = fn();
        }
        const elapsed = (performance.now() - start) / iterations;
        console.log(`  ${name.padEnd(20)}: ${elapsed.toFixed(4)} ms (result = ${lastResult})`);
    };

    runTimed('reference (oracle)', () => referenceNthPrime(n));
    runTimed('recursive', () => recursiveNthPrime(n));
    runTimed('iterative', () => iterativeNthPrime(n));
    
    const cache = new PrimeCache();
    runTimed('memoized (cached)', () => cache.nth(n));
}

if (import.meta.url === `file://${process.argv[1]}`) {
    runNthPrimeBenchmark();
}
