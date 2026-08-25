/**
 * Text and string operations in TypeScript.
 */

import { c } from './_compylr.ts';

export function wordCount(words: Array<string>): Map<string, number> {
    const counts: Map<string, number> = new Map();
    for (const word of words) {
        if (counts.has(word)) {
            counts.set(word, (counts.get(word) || 0) + 1);
        } else {
            counts.set(word, 1);
        }
    }
    return counts;
}

export function mostCommon(words: Array<string>): string {
    const counts: Map<string, number> = wordCount(words);
    let best: string = "";
    let bestCount: number = 0;
    for (const word of counts.keys()) {
        const count: number = counts.get(word) || 0;
        if (count > bestCount) {
            best = word;
            bestCount = count;
        } else if (count === bestCount) {
            if (best === "" || word < best) {
                best = word;
            }
        }
    }
    return best;
}

export function uniqueWords(words: Array<string>): Array<string> {
    const seen: Map<string, number> = new Map();
    const out: Array<string> = [];
    for (const word of words) {
        if (seen.has(word)) {
            continue;
        }
        seen.set(word, 1);
        out.push(word);
    }
    return out;
}

export function vowelLetters(): Set<string> {
    const s: Set<string> = new Set();
    s.add("a");
    s.add("e");
    s.add("i");
    s.add("o");
    s.add("u");
    return s;
}

export function countPresent(words: Array<string>, wanted: Set<string>): number {
    let total: number = 0;
    for (const word of words) {
        if (wanted.has(word)) {
            total = total + 1;
        }
    }
    return total;
}

export function isPalindrome(chars: Array<string>): boolean {
    let left: number = 0;
    let right: number = chars.length - 1;
    while (left < right) {
        if (chars[left] !== chars[right]) {
            return false;
        }
        left = left + 1;
        right = right - 1;
    }
    return true;
}

c.compyle(wordCount);
c.compyle(mostCommon);
c.compyle(uniqueWords);
c.compyle(vowelLetters);
c.compyle(countPresent);
c.compyle(isPalindrome);
