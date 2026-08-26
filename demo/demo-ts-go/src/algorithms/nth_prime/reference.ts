/**
 * Interpreted reference implementation of nth-prime as ground truth.
 */

export function referenceIsPrime(n: number): boolean {
    if (n < 2) {
        return false;
    }
    for (let d = 2; d * d <= n; d++) {
        if (n % d === 0) {
            return false;
        }
    }
    return true;
}

export function referenceNthPrime(n: number): number {
    if (n < 1) {
        return 0;
    }
    let count = 0;
    let candidate = 1;
    while (count < n) {
        candidate++;
        if (referenceIsPrime(candidate)) {
            count++;
        }
    }
    return candidate;
}
