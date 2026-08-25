export function sumList(xs: Array<number>): number {
    let sum: number = 0;
    for (const x of xs) {
        sum = sum + x;
    }
    return sum;
}
