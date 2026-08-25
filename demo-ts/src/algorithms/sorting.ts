/**
 * Sorting and searching algorithms in TypeScript.
 */

import { c } from './_compylr.ts';

export function copyOf(xs: Array<number>): Array<number> {
    const out: Array<number> = [];
    for (const x of xs) {
        out.push(x);
    }
    return out;
}

export function insertionSort(xs: Array<number>): Array<number> {
    const out: Array<number> = copyOf(xs);
    let i: number = 1;
    while (i < out.length) {
        const key: number = out[i];
        let j: number = i - 1;
        while (j >= 0) {
            if (out[j] > key) {
                out[j + 1] = out[j];
                j = j - 1;
            } else {
                break;
            }
        }
        out[j + 1] = key;
        i = i + 1;
    }
    return out;
}

export function selectionSort(xs: Array<number>): Array<number> {
    const out: Array<number> = copyOf(xs);
    let i: number = 0;
    while (i < out.length) {
        let smallest: number = i;
        let j: number = i + 1;
        while (j < out.length) {
            if (out[j] < out[smallest]) {
                smallest = j;
            }
            j = j + 1;
        }
        const held: number = out[i];
        out[i] = out[smallest];
        out[smallest] = held;
        i = i + 1;
    }
    return out;
}

export function merge(left: Array<number>, right: Array<number>): Array<number> {
    const out: Array<number> = [];
    let i: number = 0;
    let j: number = 0;
    while (i < left.length) {
        if (j >= right.length) {
            break;
        }
        if (left[i] <= right[j]) {
            out.push(left[i]);
            i = i + 1;
        } else {
            out.push(right[j]);
            j = j + 1;
        }
    }
    while (i < left.length) {
        out.push(left[i]);
        i = i + 1;
    }
    while (j < right.length) {
        out.push(right[j]);
        j = j + 1;
    }
    return out;
}

export function mergeSort(xs: Array<number>): Array<number> {
    if (xs.length <= 1) {
        return copyOf(xs);
    }
    const mid: number = Math.floor(xs.length / 2);
    const left: Array<number> = [];
    let i: number = 0;
    while (i < mid) {
        left.push(xs[i]);
        i = i + 1;
    }
    const right: Array<number> = [];
    while (i < xs.length) {
        right.push(xs[i]);
        i = i + 1;
    }
    return merge(mergeSort(left), mergeSort(right));
}

export function binarySearch(xs: Array<number>, target: number): number {
    let low: number = 0;
    let high: number = xs.length - 1;
    while (low <= high) {
        const mid: number = Math.floor((low + high) / 2);
        if (xs[mid] === target) {
            return mid;
        } else if (xs[mid] < target) {
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    return -1;
}

export function isSorted(xs: Array<number>): boolean {
    let i: number = 1;
    while (i < xs.length) {
        if (xs[i - 1] > xs[i]) {
            return false;
        }
        i = i + 1;
    }
    return true;
}

c.compyle(copyOf);
c.compyle(insertionSort);
c.compyle(selectionSort);
c.compyle(merge);
c.compyle(mergeSort);
c.compyle(binarySearch);
c.compyle(isSorted);
