/**
 * Statistics and floating point algorithms in TypeScript.
 */

import { c } from './_compylr.ts';

export function mean(xs: Array<number>): number {
    if (xs.length === 0) {
        return 0.0;
    }
    let total: number = 0.0;
    for (const x of xs) {
        total = total + x;
    }
    return total / xs.length;
}

export function averageOfCounts(counts: Array<number>): number {
    if (counts.length === 0) {
        return 0.0;
    }
    let total: number = 0;
    for (const count of counts) {
        total = total + count;
    }
    return total / counts.length;
}

export function variance(xs: Array<number>): number {
    if (xs.length === 0) {
        return 0.0;
    }
    const centre: number = mean(xs);
    let total: number = 0.0;
    for (const x of xs) {
        const deviation: number = x - centre;
        total = total + (deviation * deviation);
    }
    return total / xs.length;
}

export function squareRoot(value: number): number {
    if (value <= 0.0) {
        return 0.0;
    }
    let guess: number = value;
    let step: number = 0;
    while (step < 40) {
        guess = (guess + (value / guess)) / 2.0;
        step = step + 1;
    }
    return guess;
}

export function standardDeviation(xs: Array<number>): number {
    return squareRoot(variance(xs));
}

export function extremes(xs: Array<number>): [number, number] {
    if (xs.length === 0) {
        return [0.0, 0.0];
    }
    let low: number = xs[0];
    let high: number = xs[0];
    let i: number = 1;
    while (i < xs.length) {
        const val: number = xs[i];
        if (val < low) {
            low = val;
        }
        if (val > high) {
            high = val;
        }
        i = i + 1;
    }
    return [low, high];
}

export function normalize(xs: Array<number>): Array<number> {
    if (xs.length === 0) {
        return [];
    }
    const avg: number = mean(xs);
    const sd: number = standardDeviation(xs);
    if (sd === 0.0) {
        const zeroes: Array<number> = [];
        let i: number = 0;
        while (i < xs.length) {
            zeroes.push(0.0);
            i = i + 1;
        }
        return zeroes;
    }
    const out: Array<number> = [];
    for (const x of xs) {
        out.push((x - avg) / sd);
    }
    return out;
}

export class RunningStats {
    count: number;
    total: number;
    totalSq: number;

    constructor() {
        this.count = 0;
        this.total = 0.0;
        this.totalSq = 0.0;
    }

    add(x: number): void {
        this.count = this.count + 1;
        this.total = this.total + x;
        this.totalSq = this.totalSq + (x * x);
    }

    currentMean(): number {
        if (this.count === 0) {
            return 0.0;
        }
        return this.total / this.count;
    }

    currentVariance(): number {
        if (this.count === 0) {
            return 0.0;
        }
        const m: number = this.currentMean();
        const meanSq: number = this.totalSq / this.count;
        const v: number = meanSq - (m * m);
        if (v < 0.0) {
            return 0.0;
        }
        return v;
    }
}

c.compyle(mean);
c.compyle(averageOfCounts);
c.compyle(variance);
c.compyle(squareRoot);
c.compyle(standardDeviation);
c.compyle(extremes);
c.compyle(normalize);
c.compyle(RunningStats);
