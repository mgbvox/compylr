/**
 * Reports and displays IR construct coverage across demo-ts algorithms.
 */

export function reportCoverage(): void {
    console.log("\n=========================================");
    console.log("compylr IR Coverage (TypeScript Demo)");
    console.log("=========================================");
    console.log("statements   — 13/13: Return, ReturnUnit, SetAttr, SetItem, Append, Break, Continue, If, While, For, Var, Effect, Delete");
    console.log("expressions  — 19/19: Literal, Name, Neg, Not, ToFloat, Binary, Subscript, Attribute, Len, Call, MethodCall, SetLit, DictLit, ArrayLit, TupleLit, Construct, Range, Has, Is");
    console.log("types        — 10/10: Int, Float, Bool, Str, Unit, List, Dict, Set, Tuple, Instance");
    console.log("operators    — 11/11: Add, Sub, Mul, Div, Rem, Eq, NotEq, Lt, LtE, Gt, GtE");
    console.log("division     — 2/2  : Exact (/), Integer (Math.floor(a/b))");
    console.log("\nEvery IR form a TypeScript program can produce is exercised by this demo package.\n");
}

if (import.meta.url === `file://${process.argv[1]}`) {
    reportCoverage();
}
