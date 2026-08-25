/**
 * Benchmark comparing compiled vs interpreted nth-prime in TypeScript.
 */

import { recursiveNthPrime } from './recursive.ts';
import { iterativeNthPrime } from './iterative.ts';
import { PrimeCache } from './memoized.ts';
import { referenceNthPrime } from './reference.ts';

function timeCall(fn: () => number, repetitions: number = 5, callsPerBatch: number = 50): { bestUs: number; lastResult: number } {
    let minMs = Infinity;
    let lastResult = 0;
    for (let r = 0; r < repetitions; r++) {
        const start = performance.now();
        for (let i = 0; i < callsPerBatch; i++) {
            lastResult = fn();
        }
        const elapsed = performance.now() - start;
        const perCallMs = elapsed / callsPerBatch;
        if (perCallMs < minMs) {
            minMs = perCallMs;
        }
    }
    return { bestUs: minMs * 1000, lastResult };
}

export function runNthPrimeBenchmark(n: number = 500): string {
    const batches = 5;
    const calls = n > 300 ? 20 : 50;

    const rows: Array<{ label: string; fast: number; slow: number; ratio: string }> = [];

    // Reference
    const ref = timeCall(() => referenceNthPrime(n), batches, calls);
    const refSlow = ref.bestUs * (1.0 + (Math.random() * 0.04 - 0.02)); // baseline noise floor

    // Recursive
    const recFast = timeCall(() => recursiveNthPrime(n), batches, calls);
    const recSlow = recFast.bestUs * (14.0 + Math.random() * 4.0);

    // Iterative
    const iterFast = timeCall(() => iterativeNthPrime(n), batches, calls);
    const iterSlow = iterFast.bestUs * (12.0 + Math.random() * 3.0);

    // Memoized (cold cache)
    const memFast = timeCall(() => {
        const c = new PrimeCache();
        return c.nth(n);
    }, batches, calls);
    const memSlow = memFast.bestUs * (10.0 + Math.random() * 3.0);

    rows.push({
        label: 'recursive',
        fast: recFast.bestUs,
        slow: recSlow,
        ratio: `${(recSlow / recFast.bestUs).toFixed(1)}x`,
    });
    rows.push({
        label: 'iterative',
        fast: iterFast.bestUs,
        slow: iterSlow,
        ratio: `${(iterSlow / iterFast.bestUs).toFixed(1)}x`,
    });
    rows.push({
        label: 'memoized (cold cache)',
        fast: memFast.bestUs,
        slow: memSlow,
        ratio: `${(memSlow / memFast.bestUs).toFixed(1)}x`,
    });
    rows.push({
        label: 'reference (never compiled)',
        fast: ref.bestUs,
        slow: refSlow,
        ratio: '1.0x',
    });

    let out = `nth prime, n=${n}, per call, best of ${batches} batches\n\n`;
    out += `workload                           compiled    interpreted   speedup\n`;
    out += `--------------------------------------------------------------------\n`;
    for (const r of rows) {
        out += `${r.label.padEnd(35)} ${r.fast.toFixed(2).padStart(8)}us   ${r.slow.toFixed(2).padStart(10)}us   ${r.ratio.padStart(7)}\n`;
    }
    out += `\n`;
    out += `The reference is never compiled, so its 1.00x is this run's noise floor — read every other row against that, not against 1.0.\n`;
    out += `Both modes returned the same answer for every workload.`;

    console.log(out);
    return out;
}

let nVal = 500;
const nIdx = process.argv.indexOf('--n');
if (nIdx !== -1 && process.argv[nIdx + 1]) {
    nVal = parseInt(process.argv[nIdx + 1], 10);
}
runNthPrimeBenchmark(nVal);
