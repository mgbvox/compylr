/**
 * Integer and arithmetic algorithms in TypeScript.
 */

import { c } from './_compylr.ts';

export function floorDivide(a: number, b: number): number {
    return Math.floor(a / b);
}

export function remainder(a: number, b: number): number {
    return a % b;
}

export function gcd(a: number, b: number): number {
    let x: number = a;
    let y: number = b;
    if (x < 0) {
        x = -x;
    }
    if (y < 0) {
        y = -y;
    }
    while (y !== 0) {
        const held: number = y;
        y = x % y;
        x = held;
    }
    return x;
}

export function lcm(a: number, b: number): number {
    if (a === 0 || b === 0) {
        return 0;
    }
    let product: number = a * b;
    if (product < 0) {
        product = -product;
    }
    return Math.floor(product / gcd(a, b));
}

export function integerSqrt(n: number): number {
    if (n < 0) {
        return -1;
    }
    if (n < 2) {
        return n;
    }
    let x: number = n;
    let y: number = Math.floor((x + 1) / 2);
    while (y < x) {
        x = y;
        y = Math.floor((x + Math.floor(n / x)) / 2);
    }
    return x;
}

export function power(base: number, exponent: number): number {
    if (exponent < 0) {
        return 0;
    }
    let result: number = 1;
    let b: number = base;
    let e: number = exponent;
    while (e > 0) {
        if (e % 2 === 1) {
            result = result * b;
        }
        e = Math.floor(e / 2);
        if (e > 0) {
            b = b * b;
        }
    }
    return result;
}

export function collatzLength(n: number): number {
    if (n < 1) {
        return 0;
    }
    let steps: number = 0;
    let current: number = n;
    while (current !== 1) {
        if (current % 2 === 0) {
            current = Math.floor(current / 2);
        } else {
            current = 3 * current + 1;
        }
        steps = steps + 1;
    }
    return steps;
}

export function isPrime(n: number): boolean {
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

export function sieve(limit: number): Array<number> {
    if (limit < 3) {
        return [];
    }
    const isP: Array<boolean> = [];
    let i: number = 0;
    while (i <= limit) {
        isP.push(true);
        i = i + 1;
    }
    isP[0] = false;
    isP[1] = false;
    let p: number = 2;
    while (p * p <= limit) {
        if (isP[p]) {
            let multiple: number = p * p;
            while (multiple <= limit) {
                isP[multiple] = false;
                multiple = multiple + p;
            }
        }
        p = p + 1;
    }
    const primes: Array<number> = [];
    let n: number = 2;
    while (n <= limit) {
        if (isP[n]) {
            primes.push(n);
        }
        n = n + 1;
    }
    return primes;
}

export function digitSum(n: number): number {
    let num: number = n;
    if (num < 0) {
        num = -num;
    }
    let sum: number = 0;
    while (num > 0) {
        sum = sum + (num % 10);
        num = Math.floor(num / 10);
    }
    return sum;
}

export function divide(a: number, b: number): [number, number] {
    return [Math.floor(a / b), a % b];
}

export function toBase(n: number, base: number): Array<number> {
    let current: number = n;
    if (current < 0) {
        current = -current;
    }
    const backwards: Array<number> = [];
    if (current === 0) {
        backwards.push(0);
    }
    while (current > 0) {
        const split: [number, number] = divide(current, base);
        backwards.push(split[1]);
        current = split[0];
    }
    const digits: Array<number> = [];
    let index: number = backwards.length - 1;
    while (index >= 0) {
        digits.push(backwards[index]);
        index = index - 1;
    }
    return digits;
}

c.compyle(floorDivide);
c.compyle(remainder);
c.compyle(gcd);
c.compyle(lcm);
c.compyle(integerSqrt);
c.compyle(power);
c.compyle(collatzLength);
c.compyle(isPrime);
c.compyle(sieve);
c.compyle(digitSum);
c.compyle(divide);
c.compyle(toBase);
