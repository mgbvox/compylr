import test from 'node:test';
import assert from 'node:assert/strict';
import { c } from '../src/algorithms/_compylr.ts';
import '../src/algorithms/index.ts';

test('All demo algorithms are properly registered with compylr manager', () => {
    const registered = c.getRegistered();
    assert(registered.size >= 25, `Expected at least 25 registered functions/classes, got ${registered.size}`);
    assert(registered.has('mergeSort'));
    assert(registered.has('gcd'));
    assert(registered.has('editDistance'));
    assert(registered.has('PrimeCache'));
    assert(registered.has('UnionFind'));
});
