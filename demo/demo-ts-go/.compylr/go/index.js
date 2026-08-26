// FFI Loader for compylr_generated_319b223ac9755cf2_e6769e80
const path = require('path');
const koffi = require('koffi');

const libPath = path.join(__dirname, 'compylr_generated_319b223ac9755cf2_e6769e80.so');
const lib = koffi.load(libPath);

const native_averageOfCounts = lib.func('Call_averageOfCounts', 'int64', ['int64']);
function averageOfCounts(counts) { return native_averageOfCounts(counts); }
exports.averageOfCounts = averageOfCounts;

const native_balanced = lib.func('Call_balanced', 'int64', ['int64']);
function balanced(tokens) { return native_balanced(tokens); }
exports.balanced = balanced;

const native_bfsDistances = lib.func('Call_bfsDistances', 'int64', ['int64', 'int64']);
function bfsDistances(graph, start) { return native_bfsDistances(graph, start); }
exports.bfsDistances = bfsDistances;

const native_binarySearch = lib.func('Call_binarySearch', 'int64', ['int64', 'int64']);
function binarySearch(xs, target) { return native_binarySearch(xs, target); }
exports.binarySearch = binarySearch;

const native_coinChange = lib.func('Call_coinChange', 'int64', ['int64', 'int64']);
function coinChange(coins, amount) { return native_coinChange(coins, amount); }
exports.coinChange = coinChange;

const native_collatzLength = lib.func('Call_collatzLength', 'int64', ['int64']);
function collatzLength(n) { return native_collatzLength(n); }
exports.collatzLength = collatzLength;

const native_componentCount = lib.func('Call_componentCount', 'int64', ['int64']);
function componentCount(graph) { return native_componentCount(graph); }
exports.componentCount = componentCount;

const native_copyOf = lib.func('Call_copyOf', 'int64', ['int64']);
function copyOf(xs) { return native_copyOf(xs); }
exports.copyOf = copyOf;

const native_countPresent = lib.func('Call_countPresent', 'int64', ['int64', 'int64']);
function countPresent(words, wanted) { return native_countPresent(words, wanted); }
exports.countPresent = countPresent;

const native_depthFirstOrder = lib.func('Call_depthFirstOrder', 'int64', ['int64', 'int64']);
function depthFirstOrder(graph, start) { return native_depthFirstOrder(graph, start); }
exports.depthFirstOrder = depthFirstOrder;

const native_digitSum = lib.func('Call_digitSum', 'int64', ['int64']);
function digitSum(n) { return native_digitSum(n); }
exports.digitSum = digitSum;

const native_divide = lib.func('Call_divide', 'int64', ['int64', 'int64']);
function divide(a, b) { return native_divide(a, b); }
exports.divide = divide;

const native_editDistance = lib.func('Call_editDistance', 'int64', ['int64', 'int64']);
function editDistance(left, right) { return native_editDistance(left, right); }
exports.editDistance = editDistance;

const native_extremes = lib.func('Call_extremes', 'int64', ['int64']);
function extremes(xs) { return native_extremes(xs); }
exports.extremes = extremes;

const native_floorDivide = lib.func('Call_floorDivide', 'int64', ['int64', 'int64']);
function floorDivide(a, b) { return native_floorDivide(a, b); }
exports.floorDivide = floorDivide;

const native_gcd = lib.func('Call_gcd', 'int64', ['int64', 'int64']);
function gcd(a, b) { return native_gcd(a, b); }
exports.gcd = gcd;

const native_insertionSort = lib.func('Call_insertionSort', 'int64', ['int64']);
function insertionSort(xs) { return native_insertionSort(xs); }
exports.insertionSort = insertionSort;

const native_integerSqrt = lib.func('Call_integerSqrt', 'int64', ['int64']);
function integerSqrt(n) { return native_integerSqrt(n); }
exports.integerSqrt = integerSqrt;

const native_isPalindrome = lib.func('Call_isPalindrome', 'int64', ['int64']);
function isPalindrome(chars) { return native_isPalindrome(chars); }
exports.isPalindrome = isPalindrome;

const native_isPrime = lib.func('Call_isPrime', 'int64', ['int64']);
function isPrime(n) { return native_isPrime(n); }
exports.isPrime = isPrime;

const native_isSorted = lib.func('Call_isSorted', 'int64', ['int64']);
function isSorted(xs) { return native_isSorted(xs); }
exports.isSorted = isSorted;

const native_iterativeNotDivisible = lib.func('Call_iterativeNotDivisible', 'int64', ['int64']);
function iterativeNotDivisible(divisible) { return native_iterativeNotDivisible(divisible); }
exports.iterativeNotDivisible = iterativeNotDivisible;

const native_iterativeNthPrime = lib.func('Call_iterativeNthPrime', 'int64', ['int64']);
function iterativeNthPrime(n) { return native_iterativeNthPrime(n); }
exports.iterativeNthPrime = iterativeNthPrime;

const native_iterativePrimesUpToCount = lib.func('Call_iterativePrimesUpToCount', 'int64', ['int64']);
function iterativePrimesUpToCount(n) { return native_iterativePrimesUpToCount(n); }
exports.iterativePrimesUpToCount = iterativePrimesUpToCount;

const native_knapsack = lib.func('Call_knapsack', 'int64', ['int64', 'int64', 'int64']);
function knapsack(weights, values, capacity) { return native_knapsack(weights, values, capacity); }
exports.knapsack = knapsack;

const native_larger = lib.func('Call_larger', 'int64', ['int64', 'int64']);
function larger(a, b) { return native_larger(a, b); }
exports.larger = larger;

const native_lcm = lib.func('Call_lcm', 'int64', ['int64', 'int64']);
function lcm(a, b) { return native_lcm(a, b); }
exports.lcm = lcm;

const native_longestCommonSubsequence = lib.func('Call_longestCommonSubsequence', 'int64', ['int64', 'int64']);
function longestCommonSubsequence(left, right) { return native_longestCommonSubsequence(left, right); }
exports.longestCommonSubsequence = longestCommonSubsequence;

const native_matrixMultiply = lib.func('Call_matrixMultiply', 'int64', ['int64', 'int64']);
function matrixMultiply(a, b) { return native_matrixMultiply(a, b); }
exports.matrixMultiply = matrixMultiply;

const native_matrixTrace = lib.func('Call_matrixTrace', 'int64', ['int64']);
function matrixTrace(m) { return native_matrixTrace(m); }
exports.matrixTrace = matrixTrace;

const native_matrixTranspose = lib.func('Call_matrixTranspose', 'int64', ['int64']);
function matrixTranspose(m) { return native_matrixTranspose(m); }
exports.matrixTranspose = matrixTranspose;

const native_maxSubarraySum = lib.func('Call_maxSubarraySum', 'int64', ['int64']);
function maxSubarraySum(xs) { return native_maxSubarraySum(xs); }
exports.maxSubarraySum = maxSubarraySum;

const native_mean = lib.func('Call_mean', 'int64', ['int64']);
function mean(xs) { return native_mean(xs); }
exports.mean = mean;

const native_merge = lib.func('Call_merge', 'int64', ['int64', 'int64']);
function merge(left, right) { return native_merge(left, right); }
exports.merge = merge;

const native_mergeSort = lib.func('Call_mergeSort', 'int64', ['int64']);
function mergeSort(xs) { return native_mergeSort(xs); }
exports.mergeSort = mergeSort;

const native_mostCommon = lib.func('Call_mostCommon', 'int64', ['int64']);
function mostCommon(words) { return native_mostCommon(words); }
exports.mostCommon = mostCommon;

const native_nodeList = lib.func('Call_nodeList', 'int64', ['int64']);
function nodeList(graph) { return native_nodeList(graph); }
exports.nodeList = nodeList;

const native_normalize = lib.func('Call_normalize', 'int64', ['int64']);
function normalize(xs) { return native_normalize(xs); }
exports.normalize = normalize;

const native_power = lib.func('Call_power', 'int64', ['int64', 'int64']);
function power(base, exponent) { return native_power(base, exponent); }
exports.power = power;

const native_recursiveIsPrime = lib.func('Call_recursiveIsPrime', 'int64', ['int64']);
function recursiveIsPrime(n) { return native_recursiveIsPrime(n); }
exports.recursiveIsPrime = recursiveIsPrime;

const native_recursiveNextPrime = lib.func('Call_recursiveNextPrime', 'int64', ['int64']);
function recursiveNextPrime(after) { return native_recursiveNextPrime(after); }
exports.recursiveNextPrime = recursiveNextPrime;

const native_recursiveNthPrime = lib.func('Call_recursiveNthPrime', 'int64', ['int64']);
function recursiveNthPrime(n) { return native_recursiveNthPrime(n); }
exports.recursiveNthPrime = recursiveNthPrime;

const native_recursiveNthPrimeFrom = lib.func('Call_recursiveNthPrimeFrom', 'int64', ['int64', 'int64']);
function recursiveNthPrimeFrom(remaining, current) { return native_recursiveNthPrimeFrom(remaining, current); }
exports.recursiveNthPrimeFrom = recursiveNthPrimeFrom;

const native_remainder = lib.func('Call_remainder', 'int64', ['int64', 'int64']);
function remainder(a, b) { return native_remainder(a, b); }
exports.remainder = remainder;

const native_selectionSort = lib.func('Call_selectionSort', 'int64', ['int64']);
function selectionSort(xs) { return native_selectionSort(xs); }
exports.selectionSort = selectionSort;

const native_sieve = lib.func('Call_sieve', 'int64', ['int64']);
function sieve(limit) { return native_sieve(limit); }
exports.sieve = sieve;

const native_smaller = lib.func('Call_smaller', 'int64', ['int64', 'int64']);
function smaller(a, b) { return native_smaller(a, b); }
exports.smaller = smaller;

const native_squareRoot = lib.func('Call_squareRoot', 'int64', ['int64']);
function squareRoot(value) { return native_squareRoot(value); }
exports.squareRoot = squareRoot;

const native_standardDeviation = lib.func('Call_standardDeviation', 'int64', ['int64']);
function standardDeviation(xs) { return native_standardDeviation(xs); }
exports.standardDeviation = standardDeviation;

const native_tableOfZeros = lib.func('Call_tableOfZeros', 'int64', ['int64', 'int64']);
function tableOfZeros(rows, columns) { return native_tableOfZeros(rows, columns); }
exports.tableOfZeros = tableOfZeros;

const native_toBase = lib.func('Call_toBase', 'int64', ['int64', 'int64']);
function toBase(n, base) { return native_toBase(n, base); }
exports.toBase = toBase;

const native_topologicalSort = lib.func('Call_topologicalSort', 'int64', ['int64']);
function topologicalSort(graph) { return native_topologicalSort(graph); }
exports.topologicalSort = topologicalSort;

const native_uniqueWords = lib.func('Call_uniqueWords', 'int64', ['int64']);
function uniqueWords(words) { return native_uniqueWords(words); }
exports.uniqueWords = uniqueWords;

const native_variance = lib.func('Call_variance', 'int64', ['int64']);
function variance(xs) { return native_variance(xs); }
exports.variance = variance;

const native_vowelLetters = lib.func('Call_vowelLetters', 'int64', []);
function vowelLetters() { return native_vowelLetters(); }
exports.vowelLetters = vowelLetters;

const native_wordCount = lib.func('Call_wordCount', 'int64', ['int64']);
function wordCount(words) { return native_wordCount(words); }
exports.wordCount = wordCount;

