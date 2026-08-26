export class Accumulator {
    total: number;

    constructor(initial: number) {
        this.total = initial;
    }

    add(val: number): void {
        this.total = this.total + val;
    }

    get(): number {
        return this.total;
    }
}
