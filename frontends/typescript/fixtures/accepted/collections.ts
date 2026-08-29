// Named for the Python counterparts they pair with; see the note in arithmetic.ts.
export function sumList(xs: Array<number>): number {
    let sum: number = 0;
    for (const x of xs) {
        sum = sum + x;
    }
    return sum;
}

export function first_and_count(xs: Array<number>): number {
    let first: number = xs[0];
    let count: number = xs.length;
    return first + count;
}

export function build_list(): Array<number> {
    let xs: Array<number> = [1, 2, 3];
    return xs;
}

export function characters(s: string): number {
    return s.length;
}
