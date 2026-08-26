/**
 * Recursive nth-prime variant in TypeScript.
 */

import { c } from '../_compylr.ts';

export function recursiveIsPrime(n: number): boolean {
    if (n < 2) {
        return false;
    }
    let d: number = 2;
    while (d * d <= n) {
        if (n % d === 0) {
            return false;
        }
        d = d + 1;
    }
    return true;
}

export function recursiveNextPrime(after: number): number {
    let candidate: number = after + 1;
    let found: number = 0;
    while (found === 0) {
        if (recursiveIsPrime(candidate)) {
            found = candidate;
        }
        candidate = candidate + 1;
    }
    return found;
}

export function recursiveNthPrimeFrom(remaining: number, current: number): number {
    if (remaining < 1) {
        return current;
    }
    return recursiveNthPrimeFrom(remaining - 1, recursiveNextPrime(current));
}

export function recursiveNthPrime(n: number): number {
    if (n < 1) {
        return 0;
    }
    return recursiveNthPrimeFrom(n, 1);
}

c.compyle(recursiveIsPrime);
c.compyle(recursiveNextPrime);
c.compyle(recursiveNthPrimeFrom);
c.compyle(recursiveNthPrime);

export const nthPrime = recursiveNthPrime;
