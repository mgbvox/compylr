import test from 'node:test';
import assert from 'node:assert/strict';

import * as sorting from '../src/algorithms/sorting.ts';
import * as arithmetic from '../src/algorithms/arithmetic.ts';
import * as stats from '../src/algorithms/stats.ts';
import * as text from '../src/algorithms/text.ts';
import * as graphs from '../src/algorithms/graphs.ts';
import * as dynamic from '../src/algorithms/dynamic.ts';
import * as matrices from '../src/algorithms/matrices.ts';
import * as structures from '../src/algorithms/structures.ts';

test('Sorting algorithms agree with native sort and maintain stability', () => {
    const list = [5, 2, 8, 1, 9, 3, 7, 4, 6];
    const expected = [1, 2, 3, 4, 5, 6, 7, 8, 9];

    assert.deepEqual(sorting.insertionSort(list), expected);
    assert.deepEqual(sorting.selectionSort(list), expected);
    assert.deepEqual(sorting.mergeSort(list), expected);

    assert.equal(sorting.isSorted(expected), true);
    assert.equal(sorting.isSorted(list), false);

    assert.equal(sorting.binarySearch(expected, 7), 6);
    assert.equal(sorting.binarySearch(expected, 100), -1);
});

test('Arithmetic functions compute correct results', () => {
    assert.equal(arithmetic.gcd(48, 18), 6);
    assert.equal(arithmetic.lcm(12, 15), 60);
    assert.equal(arithmetic.integerSqrt(144), 12);
    assert.equal(arithmetic.integerSqrt(150), 12);
    assert.equal(arithmetic.power(2, 10), 1024);
    assert.equal(arithmetic.collatzLength(6), 8);
    assert.equal(arithmetic.isPrime(17), true);
    assert.equal(arithmetic.isPrime(18), false);
    assert.deepEqual(arithmetic.sieve(20), [2, 3, 5, 7, 11, 13, 17, 19]);
    assert.equal(arithmetic.digitSum(12345), 15);
    assert.deepEqual(arithmetic.divide(17, 5), [3, 2]);
    assert.deepEqual(arithmetic.toBase(255, 16), [15, 15]);
});

test('Statistics functions handle floating point precision and aggregation', () => {
    const data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    assert.equal(stats.mean(data), 5.0);
    assert.equal(stats.averageOfCounts([1, 2, 3, 4]), 2.5);
    assert.equal(stats.variance(data), 4.0);
    assert.equal(stats.standardDeviation(data), 2.0);
    assert.deepEqual(stats.extremes(data), [2.0, 9.0]);

    const running = new stats.RunningStats();
    for (const x of data) {
        running.add(x);
    }
    assert.equal(running.currentMean(), 5.0);
    assert.equal(running.currentVariance(), 4.0);
});

test('Text operations perform correct frequency, word set and palindrome checks', () => {
    const words = ["apple", "banana", "apple", "cherry", "banana", "apple"];
    const counts = text.wordCount(words);
    assert.equal(counts.get("apple"), 3);
    assert.equal(counts.get("banana"), 2);
    assert.equal(counts.get("cherry"), 1);

    assert.equal(text.mostCommon(words), "apple");
    assert.deepEqual(text.uniqueWords(words), ["apple", "banana", "cherry"]);

    const vowels = text.vowelLetters();
    assert.equal(text.countPresent(["a", "b", "c", "e"], vowels), 2);

    assert.equal(text.isPalindrome(["r", "a", "c", "e", "c", "a", "r"]), true);
    assert.equal(text.isPalindrome(["h", "e", "l", "l", "o"]), false);
});

test('Graph algorithms find paths, topological order and components', () => {
    const graph = new Map<number, Array<number>>();
    graph.set(1, [2, 3]);
    graph.set(2, [4]);
    graph.set(3, [4]);
    graph.set(4, []);

    assert.deepEqual(graphs.nodeList(graph), [1, 2, 3, 4]);

    const distances = graphs.bfsDistances(graph, 1);
    assert.equal(distances.get(1), 0);
    assert.equal(distances.get(2), 1);
    assert.equal(distances.get(3), 1);
    assert.equal(distances.get(4), 2);

    const topo = graphs.topologicalSort(graph);
    assert.deepEqual(topo, [1, 2, 3, 4]);

    assert.equal(graphs.componentCount(graph), 1);
});

test('Dynamic programming algorithms solve classic subproblems', () => {
    const left = ["k", "i", "t", "t", "e", "n"];
    const right = ["s", "i", "t", "t", "i", "n", "g"];
    assert.equal(dynamic.editDistance(left, right), 3);

    const seqA = [1, 2, 3, 4, 1];
    const seqB = [3, 4, 1, 2, 1, 3];
    assert.equal(dynamic.longestCommonSubsequence(seqA, seqB), 3);

    assert.equal(dynamic.coinChange([1, 2, 5], 11), 3);
    assert.equal(dynamic.knapsack([2, 3, 4, 5], [3, 4, 5, 6], 5), 7);
    assert.equal(dynamic.maxSubarraySum([-2, 1, -3, 4, -1, 2, 1, -5, 4]), 6);
});

test('Matrix operations multiply, transpose and calculate trace', () => {
    const a = [[1, 2], [3, 4]];
    const b = [[2, 0], [1, 2]];
    const expectedMul = [[4, 4], [10, 8]];

    assert.deepEqual(matrices.matrixMultiply(a, b), expectedMul);
    assert.deepEqual(matrices.matrixTranspose(a), [[1, 3], [2, 4]]);
    assert.equal(matrices.matrixTrace(a), 5);
});

test('Data structures preserve state and operations', () => {
    const stack = new structures.IntStack();
    stack.push(10);
    stack.push(20);
    assert.equal(stack.depth(), 2);
    assert.equal(stack.peek(), 20);
    assert.equal(stack.pop(), 20);
    assert.equal(stack.pop(), 10);
    assert.equal(stack.pop(), 0);

    assert.equal(structures.balanced([1, 2, -2, -1]), true);
    assert.equal(structures.balanced([1, 2, -1, -2]), false);

    const uf = new structures.UnionFind(5);
    assert.equal(uf.connected(0, 1), false);
    uf.union(0, 1);
    uf.union(1, 2);
    assert.equal(uf.connected(0, 2), true);
    assert.equal(uf.setCount(), 3);
});
