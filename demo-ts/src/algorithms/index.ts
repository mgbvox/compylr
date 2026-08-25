/**
 * Main entry point for the TypeScript algorithms demo.
 */

import { runAllBenchmarks } from './benchmark.ts';
import { reportCoverage } from './ir_coverage.ts';

export * from './sorting.ts';
export * from './arithmetic.ts';
export * from './stats.ts';
export * from './text.ts';
export * from './graphs.ts';
export * from './dynamic.ts';
export * from './matrices.ts';
export * from './structures.ts';
export * as nthPrime from './nth_prime/index.ts';

export function main(): void {
    console.log("Running compylr TypeScript Algorithms Demo...");
    reportCoverage();
    runAllBenchmarks();
}

if (import.meta.url === `file://${process.argv[1]}`) {
    main();
}
