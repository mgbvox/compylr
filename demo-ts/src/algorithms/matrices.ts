/**
 * Matrix multiplication and operations in TypeScript.
 */

import { c } from './_compylr.ts';
import { tableOfZeros } from './dynamic.ts';

export function matrixMultiply(a: Array<Array<number>>, b: Array<Array<number>>): Array<Array<number>> {
    if (a.length === 0 || b.length === 0) {
        return [];
    }
    const rowsA: number = a.length;
    const colsA: number = a[0].length;
    const colsB: number = b[0].length;
    const out: Array<Array<number>> = tableOfZeros(rowsA, colsB);
    let i: number = 0;
    while (i < rowsA) {
        let j: number = 0;
        while (j < colsB) {
            let sum: number = 0;
            let k: number = 0;
            while (k < colsA) {
                sum = sum + a[i][k] * b[k][j];
                k = k + 1;
            }
            out[i][j] = sum;
            j = j + 1;
        }
        i = i + 1;
    }
    return out;
}

export function matrixTranspose(m: Array<Array<number>>): Array<Array<number>> {
    if (m.length === 0) {
        return [];
    }
    const rows: number = m.length;
    const cols: number = m[0].length;
    const out: Array<Array<number>> = tableOfZeros(cols, rows);
    let i: number = 0;
    while (i < rows) {
        let j: number = 0;
        while (j < cols) {
            out[j][i] = m[i][j];
            j = j + 1;
        }
        i = i + 1;
    }
    return out;
}

export function matrixTrace(m: Array<Array<number>>): number {
    let trace: number = 0;
    let i: number = 0;
    while (i < m.length) {
        if (i < m[i].length) {
            trace = trace + m[i][i];
        }
        i = i + 1;
    }
    return trace;
}

c.compyle(matrixMultiply);
c.compyle(matrixTranspose);
c.compyle(matrixTrace);
