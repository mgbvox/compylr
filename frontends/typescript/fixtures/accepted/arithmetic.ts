// Members named for their Python counterparts rather than in TypeScript's own casing. The
// cross-language divergence check pairs members BY NAME, so `difference` here and `difference`
// in the Python corpus are the pair; renaming this to `computeDifference` would not be a style
// fix, it would silently drop the pair and lower the recorded total by measuring less.
export function add(a: number, b: number): number {
    return a + b;
}

export function compute(x: number, y: number): number {
    return (x * y) - (x + y);
}

export function difference(a: number, b: number): number {
    return a - b;
}

export function product(a: number, b: number): number {
    return a * b;
}

export function halve(a: number): number {
    return a / 2;
}

export function modulo(a: number, b: number): number {
    return a % b;
}

export function negate(a: number): number {
    return -a;
}
