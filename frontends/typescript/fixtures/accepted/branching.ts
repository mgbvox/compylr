// Named for the Python counterparts they pair with; see the note in arithmetic.ts.
export function sign(n: number): number {
    if (n > 0) {
        return 1;
    } else if (n < 0) {
        return -1;
    } else {
        return 0;
    }
}

export function clamp(n: number, low: number, high: number): number {
    if (n < low) {
        return low;
    }
    if (n > high) {
        return high;
    }
    return n;
}

export function describe(n: number): string {
    let label: string = "small";
    if (n > 100) {
        label = "large";
    }
    return label;
}
