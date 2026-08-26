/**
 * Dynamic programming algorithms in TypeScript.
 */

import { c } from './_compylr.ts';

export function tableOfZeros(rows: number, columns: number): Array<Array<number>> {
    const table: Array<Array<number>> = [];
    let r: number = 0;
    while (r < rows) {
        const line: Array<number> = [];
        let c: number = 0;
        while (c < columns) {
            line.push(0);
            c = c + 1;
        }
        table.push(line);
        r = r + 1;
    }
    return table;
}

export function smaller(a: number, b: number): number {
    if (a < b) {
        return a;
    }
    return b;
}

export function larger(a: number, b: number): number {
    if (a > b) {
        return a;
    }
    return b;
}

export function editDistance(left: Array<string>, right: Array<string>): number {
    const rows: number = left.length;
    const columns: number = right.length;
    const table: Array<Array<number>> = tableOfZeros(rows + 1, columns + 1);
    let i: number = 0;
    while (i <= rows) {
        table[i][0] = i;
        i = i + 1;
    }
    let j: number = 0;
    while (j <= columns) {
        table[0][j] = j;
        j = j + 1;
    }
    i = 1;
    while (i <= rows) {
        j = 1;
        while (j <= columns) {
            if (left[i - 1] === right[j - 1]) {
                table[i][j] = table[i - 1][j - 1];
            } else {
                const best: number = smaller(table[i - 1][j], table[i][j - 1]);
                table[i][j] = smaller(best, table[i - 1][j - 1]) + 1;
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return table[rows][columns];
}

export function longestCommonSubsequence(left: Array<number>, right: Array<number>): number {
    const rows: number = left.length;
    const columns: number = right.length;
    const table: Array<Array<number>> = tableOfZeros(rows + 1, columns + 1);
    let i: number = 1;
    while (i <= rows) {
        let j: number = 1;
        while (j <= columns) {
            if (left[i - 1] === right[j - 1]) {
                table[i][j] = table[i - 1][j - 1] + 1;
            } else {
                table[i][j] = larger(table[i - 1][j], table[i][j - 1]);
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return table[rows][columns];
}

export function coinChange(coins: Array<number>, amount: number): number {
    if (amount <= 0) {
        return 0;
    }
    const infinity: number = amount + 1;
    const dp: Array<number> = [];
    dp.push(0);
    let a: number = 1;
    while (a <= amount) {
        dp.push(infinity);
        a = a + 1;
    }
    for (const coin of coins) {
        let current: number = coin;
        while (current <= amount) {
            dp[current] = smaller(dp[current], dp[current - coin] + 1);
            current = current + 1;
        }
    }
    if (dp[amount] >= infinity) {
        return -1;
    }
    return dp[amount];
}

export function knapsack(weights: Array<number>, values: Array<number>, capacity: number): number {
    const n: number = weights.length;
    const table: Array<Array<number>> = tableOfZeros(n + 1, capacity + 1);
    let i: number = 1;
    while (i <= n) {
        const w: number = weights[i - 1];
        const v: number = values[i - 1];
        let c: number = 0;
        while (c <= capacity) {
            if (w <= c) {
                table[i][c] = larger(table[i - 1][c], table[i - 1][c - w] + v);
            } else {
                table[i][c] = table[i - 1][c];
            }
            c = c + 1;
        }
        i = i + 1;
    }
    return table[n][capacity];
}

export function maxSubarraySum(xs: Array<number>): number {
    if (xs.length === 0) {
        return 0;
    }
    let maxSoFar: number = xs[0];
    let currentMax: number = xs[0];
    let i: number = 1;
    while (i < xs.length) {
        const x: number = xs[i];
        currentMax = larger(x, currentMax + x);
        maxSoFar = larger(maxSoFar, currentMax);
        i = i + 1;
    }
    return maxSoFar;
}

c.compyle(tableOfZeros);
c.compyle(smaller);
c.compyle(larger);
c.compyle(editDistance);
c.compyle(longestCommonSubsequence);
c.compyle(coinChange);
c.compyle(knapsack);
c.compyle(maxSubarraySum);
