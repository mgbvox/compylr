/**
 * Iterative nth-prime variant in TypeScript.
 */

import { c } from '../_compylr.ts';

export function iterativeNotDivisible(divisible: boolean): boolean {
    if (divisible) {
        return false;
    }
    return true;
}

export function iterativePrimesUpToCount(n: number): Array<number> {
    const found: Array<number> = [];
    let candidate: number = 2;
    while (found.length < n) {
        let divisible: boolean = false;
        for (const p of found) {
            if (p * p > candidate) {
                break;
            }
            if (candidate % p === 0) {
                divisible = true;
                break;
            }
        }
        if (iterativeNotDivisible(divisible)) {
            found.push(candidate);
        }
        candidate = candidate + 1;
    }
    return found;
}

export function iterativeNthPrime(n: number): number {
    if (n < 1) {
        return 0;
    }
    const found: Array<number> = iterativePrimesUpToCount(n);
    return found[n - 1];
}

c.compyle(iterativeNotDivisible);
c.compyle(iterativePrimesUpToCount);
c.compyle(iterativeNthPrime);

export const nthPrime = iterativeNthPrime;
