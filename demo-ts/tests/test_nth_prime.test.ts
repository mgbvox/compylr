import test from 'node:test';
import assert from 'node:assert/strict';

import { recursiveNthPrime } from '../src/algorithms/nth_prime/recursive.ts';
import { iterativeNthPrime } from '../src/algorithms/nth_prime/iterative.ts';
import { PrimeCache } from '../src/algorithms/nth_prime/memoized.ts';
import { referenceNthPrime } from '../src/algorithms/nth_prime/reference.ts';

test('nth-prime variants agree with each other and with interpreted reference oracle', () => {
    const knownPrimes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31];
    const cache = new PrimeCache();

    for (let i = 1; i <= knownPrimes.length; i++) {
        const expected = knownPrimes[i - 1];
        assert.equal(referenceNthPrime(i), expected, `Reference failed at n=${i}`);
        assert.equal(recursiveNthPrime(i), expected, `Recursive failed at n=${i}`);
        assert.equal(iterativeNthPrime(i), expected, `Iterative failed at n=${i}`);
        assert.equal(cache.nth(i), expected, `Memoized failed at n=${i}`);
    }

    for (let n = 15; n <= 35; n++) {
        const expected = referenceNthPrime(n);
        assert.equal(recursiveNthPrime(n), expected);
        assert.equal(iterativeNthPrime(n), expected);
        assert.equal(cache.nth(n), expected);
    }
});

test('Memoized PrimeCache correctly caches and tracks hits', () => {
    const cache = new PrimeCache();
    assert.equal(cache.hitCount(), 0);
    assert.equal(cache.knownCount(), 0);

    const prime10 = cache.nth(10);
    assert.equal(prime10, 29);
    assert.equal(cache.hitCount(), 0);
    assert.equal(cache.knownCount(), 1);

    const hitResult = cache.nth(10);
    assert.equal(hitResult, 29);
    assert.equal(cache.hitCount(), 1);
});
