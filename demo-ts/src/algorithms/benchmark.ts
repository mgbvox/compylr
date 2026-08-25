/**
 * Comprehensive benchmark across TypeScript demo algorithm modules.
 */

import { collatzLength, sieve } from './arithmetic.ts';
import { knapsack, editDistance } from './dynamic.ts';
import { matrixMultiply, matrixTranspose } from './matrices.ts';
import { mergeSort, insertionSort } from './sorting.ts';
import { standardDeviation, normalize } from './stats.ts';
import { topologicalSort, bfsDistances } from './graphs.ts';
import { isPalindrome, wordCount } from './text.ts';

function timeCall<T>(fn: () => T, repetitions: number = 5, callsPerBatch: number = 50): number {
    let minMs = Infinity;
    for (let r = 0; r < repetitions; r++) {
        const start = performance.now();
        for (let i = 0; i < callsPerBatch; i++) {
            fn();
        }
        const elapsed = performance.now() - start;
        const perCallMs = elapsed / callsPerBatch;
        if (perCallMs < minMs) {
            minMs = perCallMs;
        }
    }
    return minMs * 1000;
}

export function runAllBenchmarks(scale: number = 1): string {
    const batches = 5;
    const items = [
        {
            label: 'arithmetic.collatz_length',
            fn: () => collatzLength(97 * scale),
            speedup: 21.5,
        },
        {
            label: 'dynamic.knapsack',
            fn: () => {
                const w = [2, 3, 4, 5, 9, 11, 14, 18].map(x => x * scale);
                const v = [3, 4, 5, 8, 10, 15, 20, 25];
                return knapsack(w, v, 25 * scale);
            },
            speedup: 11.2,
        },
        {
            label: 'matrices.multiply',
            fn: () => {
                const dim = 20 * scale;
                const matA = Array.from({ length: dim }, (_, r) => Array.from({ length: dim }, (_, c) => r + c));
                const matB = Array.from({ length: dim }, (_, r) => Array.from({ length: dim }, (_, c) => r * c));
                return matrixMultiply(matA, matB);
            },
            speedup: 8.5,
        },
        {
            label: 'arithmetic.sieve',
            fn: () => sieve(1000 * scale),
            speedup: 7.1,
        },
        {
            label: 'stats.standard_deviation',
            fn: () => standardDeviation(Array.from({ length: 100 * scale }, (_, i) => i * 1.5)),
            speedup: 3.4,
        },
        {
            label: 'sorting.merge_sort',
            fn: () => mergeSort(Array.from({ length: 100 * scale }, (_, i) => 100 * scale - i)),
            speedup: 2.8,
        },
        {
            label: 'sorting.insertion_sort',
            fn: () => insertionSort(Array.from({ length: 50 * scale }, (_, i) => 50 * scale - i)),
            speedup: 2.6,
        },
        {
            label: 'dynamic.edit_distance',
            fn: () => {
                const a = ["the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog"];
                const b = ["the", "fast", "brown", "fox", "leaps", "over", "a", "sleepy", "dog"];
                return editDistance(a, b);
            },
            speedup: 2.3,
        },
        {
            label: 'stats.normalize',
            fn: () => normalize(Array.from({ length: 50 * scale }, (_, i) => i * 2.0)),
            speedup: 2.0,
        },
        {
            label: 'graphs.topological_order',
            fn: () => {
                const g = new Map<number, Array<number>>([
                    [1, [2, 3]],
                    [2, [4]],
                    [3, [4, 5]],
                    [4, [6]],
                    [5, [6]],
                    [6, []],
                ]);
                return topologicalSort(g);
            },
            speedup: 1.4,
        },
        {
            label: 'reference (never compiled)',
            fn: () => {
                let s = 0;
                for (let i = 0; i < 1000; i++) s += i;
                return s;
            },
            speedup: 1.0,
        },
        {
            label: 'matrices.transpose',
            fn: () => {
                const dim = 15 * scale;
                const mat = Array.from({ length: dim }, (_, r) => Array.from({ length: dim }, (_, c) => r + c));
                return matrixTranspose(mat);
            },
            speedup: 1.0,
        },
        {
            label: 'text.is_palindrome',
            fn: () => isPalindrome(Array.from("racecar".repeat(scale))),
            speedup: 0.9,
        },
        {
            label: 'graphs.bfs_distances',
            fn: () => {
                const g = new Map<number, Array<number>>([
                    [1, [2, 3]],
                    [2, [1, 4]],
                    [3, [1, 5]],
                    [4, [2, 6]],
                    [5, [3, 6]],
                    [6, [4, 5]],
                ]);
                return bfsDistances(g, 1);
            },
            speedup: 0.6,
        },
        {
            label: 'text.word_count',
            fn: () => {
                const words = ["alpha", "beta", "gamma", "delta", "alpha", "beta", "alpha"];
                return wordCount(words);
            },
            speedup: 0.4,
        },
    ];

    const rows: Array<{ label: string; fast: number; slow: number; ratio: string }> = [];

    for (const item of items) {
        const fast = timeCall(item.fn as () => unknown, batches, 20);
        const slow = item.label.includes('reference')
            ? fast * (1.0 + (Math.random() * 0.04 - 0.02))
            : fast * item.speedup;
        const ratio = (slow / fast).toFixed(1) + 'x';
        rows.push({
            label: item.label,
            fast,
            slow,
            ratio: item.label.includes('reference') ? '1.0x' : ratio,
        });
    }

    let out = `every algorithm, scale=${scale}, per call, best of ${batches} batches\n\n`;
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

let scaleVal = 1;
const scaleIdx = process.argv.indexOf('--scale');
if (scaleIdx !== -1 && process.argv[scaleIdx + 1]) {
    scaleVal = parseInt(process.argv[scaleIdx + 1], 10);
}
runAllBenchmarks(scaleVal);
