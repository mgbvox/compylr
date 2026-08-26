/**
 * Graph algorithms over adjacency maps in TypeScript.
 */

import { c } from './_compylr.ts';
import { mergeSort } from './sorting.ts';

export function nodeList(graph: Map<number, Array<number>>): Array<number> {
    const seen: Map<number, number> = new Map();
    const raw: Array<number> = [];
    for (const node of graph.keys()) {
        if (!seen.has(node)) {
            seen.set(node, 1);
            raw.push(node);
        }
        const neighbours = graph.get(node) || [];
        for (const neighbour of neighbours) {
            if (seen.has(neighbour)) {
                continue;
            }
            seen.set(neighbour, 1);
            raw.push(neighbour);
        }
    }
    return mergeSort(raw);
}

export function bfsDistances(graph: Map<number, Array<number>>, start: number): Map<number, number> {
    const distance: Map<number, number> = new Map();
    distance.set(start, 0);
    const queue: Array<number> = [];
    queue.push(start);
    let head: number = 0;
    while (head < queue.length) {
        const node: number = queue[head];
        head = head + 1;
        if (!graph.has(node)) {
            continue;
        }
        const neighbours = graph.get(node) || [];
        const currentDist: number = distance.get(node) || 0;
        for (const neighbour of neighbours) {
            if (distance.has(neighbour)) {
                continue;
            }
            distance.set(neighbour, currentDist + 1);
            queue.push(neighbour);
        }
    }
    return distance;
}

export function depthFirstOrder(graph: Map<number, Array<number>>, start: number): Array<number> {
    const order: Array<number> = [];
    const seen: Map<number, number> = new Map();
    const stack: Array<number> = [];
    stack.push(start);
    let top: number = 1;
    while (top > 0) {
        top = top - 1;
        const node: number = stack[top];
        if (seen.has(node)) {
            continue;
        }
        seen.set(node, 1);
        order.push(node);
        if (!graph.has(node)) {
            continue;
        }
        const neighbours = graph.get(node) || [];
        let index: number = neighbours.length - 1;
        while (index >= 0) {
            const neighbour: number = neighbours[index];
            if (!seen.has(neighbour)) {
                if (top < stack.length) {
                    stack[top] = neighbour;
                } else {
                    stack.push(neighbour);
                }
                top = top + 1;
            }
            index = index - 1;
        }
    }
    return order;
}

export function topologicalSort(graph: Map<number, Array<number>>): Array<number> {
    const inDegree: Map<number, number> = new Map();
    const nodes: Array<number> = nodeList(graph);
    for (const n of nodes) {
        inDegree.set(n, 0);
    }
    for (const u of graph.keys()) {
        const neighbours = graph.get(u) || [];
        for (const v of neighbours) {
            inDegree.set(v, (inDegree.get(v) || 0) + 1);
        }
    }
    const ready: Array<number> = [];
    for (const n of nodes) {
        if ((inDegree.get(n) || 0) === 0) {
            ready.push(n);
        }
    }
    const order: Array<number> = [];
    let head: number = 0;
    while (head < ready.length) {
        const u: number = ready[head];
        head = head + 1;
        order.push(u);
        if (graph.has(u)) {
            const neighbours = graph.get(u) || [];
            for (const v of neighbours) {
                const deg: number = (inDegree.get(v) || 0) - 1;
                inDegree.set(v, deg);
                if (deg === 0) {
                    ready.push(v);
                }
            }
        }
    }
    if (order.length !== nodes.length) {
        return [];
    }
    return order;
}

export function componentCount(graph: Map<number, Array<number>>): number {
    const nodes: Array<number> = nodeList(graph);
    const visited: Map<number, number> = new Map();
    let count: number = 0;
    for (const node of nodes) {
        if (visited.has(node)) {
            continue;
        }
        count = count + 1;
        const reached: Array<number> = depthFirstOrder(graph, node);
        for (const r of reached) {
            visited.set(r, 1);
        }
    }
    return count;
}

c.compyle(nodeList);
c.compyle(bfsDistances);
c.compyle(depthFirstOrder);
c.compyle(topologicalSort);
c.compyle(componentCount);
