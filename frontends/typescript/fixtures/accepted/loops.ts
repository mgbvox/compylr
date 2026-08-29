// Named for the Python counterparts they pair with; see the note in arithmetic.ts.
//
// Python's `range()` loops -- `total`, `countdown`, `stepped` -- have no counterpart here. The
// TypeScript frontend accepts `while` and `for...of` and not the three-clause `for`, so there is
// no way to write them in the accepted subset.
//
// `halve_until_odd` has no counterpart either, for a different reason: it reassigns its own
// parameter, which Python accepts and this frontend refuses outright. Both are real asymmetries
// rather than oversights, and the divergence table reports them by having no pair for those names.
export function sumTo(n: number): number {
    let total: number = 0;
    let i: number = 1;
    while (i <= n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}

export function first_over(xs: Array<number>, limit: number): number {
    for (const x of xs) {
        if (x > limit) {
            return x;
        }
    }
    return -1;
}

export function skip_negatives(xs: Array<number>): number {
    let kept: number = 0;
    for (const x of xs) {
        if (x < 0) {
            continue;
        }
        kept = kept + 1;
    }
    return kept;
}

