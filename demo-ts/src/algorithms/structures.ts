/**
 * Stateful data structures and classes in TypeScript.
 */

import { c } from './_compylr.ts';

export class IntStack {
    slots: Array<number>;
    height: number;

    constructor() {
        this.slots = [];
        this.height = 0;
    }

    push(value: number): void {
        if (this.height < this.slots.length) {
            this.slots[this.height] = value;
        } else {
            this.slots.push(value);
        }
        this.height = this.height + 1;
    }

    pop(): number {
        if (this.height === 0) {
            return 0;
        }
        this.height = this.height - 1;
        return this.slots[this.height];
    }

    peek(): number {
        if (this.height === 0) {
            return 0;
        }
        return this.slots[this.height - 1];
    }

    depth(): number {
        return this.height;
    }
}

export function balanced(tokens: Array<number>): boolean {
    const stack: IntStack = new IntStack();
    for (const token of tokens) {
        if (token > 0) {
            stack.push(token);
        } else {
            if (stack.depth() === 0) {
                return false;
            }
            if (stack.pop() + token !== 0) {
                return false;
            }
        }
    }
    return stack.depth() === 0;
}

export class UnionFind {
    parent: Array<number>;
    rank: Array<number>;
    components: number;

    constructor(size: number) {
        this.parent = [];
        this.rank = [];
        this.components = size;
        let i: number = 0;
        while (i < size) {
            this.parent.push(i);
            this.rank.push(0);
            i = i + 1;
        }
    }

    find(p: number): number {
        let root: number = p;
        while (root !== this.parent[root]) {
            root = this.parent[root];
        }
        let curr: number = p;
        while (curr !== root) {
            const next: number = this.parent[curr];
            this.parent[curr] = root;
            curr = next;
        }
        return root;
    }

    union(p: number, q: number): void {
        const rootP: number = this.find(p);
        const rootQ: number = this.find(q);
        if (rootP === rootQ) {
            return;
        }
        if (this.rank[rootP] < this.rank[rootQ]) {
            this.parent[rootP] = rootQ;
        } else if (this.rank[rootP] > this.rank[rootQ]) {
            this.parent[rootQ] = rootP;
        } else {
            this.parent[rootQ] = rootP;
            this.rank[rootP] = this.rank[rootP] + 1;
        }
        this.components = this.components - 1;
    }

    connected(p: number, q: number): boolean {
        return this.find(p) === this.find(q);
    }

    setCount(): number {
        return this.components;
    }
}

c.compyle(IntStack);
c.compyle(balanced);
c.compyle(UnionFind);
