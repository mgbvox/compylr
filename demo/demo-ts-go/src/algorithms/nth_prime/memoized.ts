/**
 * Memoized PrimeCache nth-prime variant in TypeScript.
 */

import { c } from '../_compylr.ts';

export class PrimeCache {
    known: Map<number, number>;
    hits: number;

    constructor() {
        this.known = new Map();
        this.hits = 0;
    }

    isPrime(n: number): boolean {
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

    nth(n: number): number {
        if (n < 1) {
            return 0;
        }
        if (this.known.has(n)) {
            this.hits = this.hits + 1;
            return this.known.get(n)!;
        }
        let found: number = 0;
        let candidate: number = 1;
        while (found < n) {
            candidate = candidate + 1;
            if (this.isPrime(candidate)) {
                found = found + 1;
            }
        }
        this.known.set(n, candidate);
        return candidate;
    }

    hitCount(): number {
        return this.hits;
    }

    knownCount(): number {
        return this.known.size;
    }
}

c.compyle(PrimeCache);
