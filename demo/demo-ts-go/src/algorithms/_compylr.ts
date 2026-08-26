/**
 * compylr runtime registration manager for TypeScript.
 */

export class CompylrManager {
    private registered: Map<string, Function> = new Map();

    compyle<T extends Function>(target: T): T {
        if (target.name) {
            this.registered.set(target.name, target);
        }
        return target;
    }

    getRegistered(): ReadonlyMap<string, Function> {
        return this.registered;
    }
}

export const c = new CompylrManager();
export const compyle = <T extends Function>(fn: T): T => c.compyle(fn);
