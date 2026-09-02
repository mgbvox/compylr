# TypeScript frontend audit — evidence transcripts

Dimension: crates/compylr-frontend-typescript/. Issue #37 (already filed) confirmed `/` lowers to
integer division regardless of operand type. This file records additional, newly-confirmed
divergences found by running the CLI against crafted fixtures.

## 1. `&&`, `||`, `??` all silently discard their right operand

crates/compylr-frontend-typescript/src/lower.rs:1207-1218:
```rust
OxcExpr::LogicalExpression(log) => {
    let (left_expr, left_ty) = lower_expr(&log.left, ctx)?;
    let (_, right_ty) = lower_expr(&log.right, ctx)?;
    let ty = if left_ty == Ty::Bool && right_ty == Ty::Bool {
        Ty::Bool
    } else if left_ty != Ty::Unit {
        left_ty
    } else {
        right_ty
    };
    Ok((left_expr, ty))
}
```
`log.operator` (`&&` vs `||` vs `??`) is never read, and the returned `Expr` is always
`left_expr` — the right operand is lowered only to steal its type, then thrown away.

Command:
```
$ cat context/logical.ts
export function orTest(a: boolean, b: boolean): boolean {
    return a || b;
}
export function andTest(a: boolean, b: boolean): boolean {
    return a && b;
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/logical.ts
pub fn andTest(a: bool, b: bool) -> Result<bool, RuntimeError> {
    Ok(a)
}
pub fn orTest(a: bool, b: bool) -> Result<bool, RuntimeError> {
    Ok(a)
}
```
`orTest(false, true)` must be `true` in TypeScript; the compiled function returns `false`.
`andTest(true, false)` must be `false`; the compiled function returns `true`. Every `&&`/`||`/`??`
expression in the accepted subset is wrong whenever the two operands would actually differ in
truth value — this is not an edge case, it is the operators' entire reason to exist.

Confirmed the same for `??`:
```
$ cat context/nullish.ts
export function nullishTest(a: number, b: number): number {
    return a ?? b;
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/nullish.ts
pub fn nullishTest(a: i64, b: i64) -> Result<i64, RuntimeError> {
    Ok(a)
}
```

No fixture under `frontends/typescript/fixtures/accepted/` uses `&&`, `||`, or `??` — the
corpus that is supposed to prove the subset never exercises this path.

## 2. `IndexOrigin::FromEitherEnd` declared for TypeScript, contradicting the IR's own doc comment

crates/compylr-frontend-typescript/src/frontend.rs:33-36:
```rust
sequence_index: SequenceIndex {
    origin: IndexOrigin::FromEitherEnd,
    checked: Checked::Reported,
},
```

But `compylr-ir`'s own definition of the enum it is instantiating says the opposite by name,
crates/compylr-ir/src/ir.rs:303-311:
```rust
/// How a negative offset into a sequence is resolved.
///
/// Python counts backwards from the end, so `xs[-1]` is the last element. Go, C++, Rust, and
/// TypeScript do not: a negative offset is out of range, undefined, or an enormous positive number.
/// Same operation, two conventions — the shape [`Rounding`] has.
pub enum IndexOrigin {
    /// A negative offset counts backwards from the end: `xs[-1]` is the last element.
    FromEitherEnd,
    /// A negative offset is out of range.
    FromStart,
}
```
The doc comment explicitly names TypeScript as a `FromStart` language, in the same enum the
TypeScript frontend sets to `FromEitherEnd`. This is a direct, in-repo contradiction, not just a
mismatch against outside knowledge of JS.

Confirmed by running the CLI:
```
$ cat context/neg_index.ts
export function lastOf(xs: Array<number>): number {
    return xs[-1];
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/neg_index.ts
pub fn lastOf(xs: Vec<i64>) -> Result<i64, RuntimeError> {
    Ok(py_subscript(&(xs), &(-1i64), IndexOrigin::FromEitherEnd)?)
}
```
Real JavaScript/TypeScript: `[10, 20, 30][-1]` is `undefined` (property `"-1"` does not exist on
the array). Compiled compylr: `lastOf([10, 20, 30])` returns `30` — the *last* element, i.e.
Python semantics, not TypeScript semantics. Every array/string negative-index read in the accepted
TypeScript subset is wrong.

(Separately, `checked: Checked::Reported` also means out-of-range access raises a `RuntimeError`
in generated code; real JS/TS never throws on an out-of-range array index, it returns `undefined`.
`Checked` only has `Reported`/`Unchecked` so there may be no clean representation of "returns a
sentinel, never errors" today — noted as a second-order gap on the same axis, lower confidence
than the `FromEitherEnd`/`FromStart` mistake above since it may be a known IR limitation rather
than a frontend authoring error.)

## 3. Float literals passed as call/array/push arguments are silently truncated to `i64`, producing generated Rust that fails to compile

crates/compylr-frontend-typescript/src/lower.rs has three separate call sites that special-case
`Argument::NumericLiteral` / `ArrayExpressionElement::NumericLiteral` ahead of the general
`lower_expr` path and hard-code `Literal::Int(n.value as i64)`, never checking whether the literal
is fractional (contrast with the general numeric-literal path at lower.rs:1021-1027, which does
check `num.value.fract() == 0.0`):

- lower.rs:1319-1326 (call arguments)
- lower.rs:1223-1225 (array literal elements)
- lower.rs:965-969 (`.push`/`.add` arguments)

Confirmed for call arguments:
```
$ cat context/floatarg.ts
export function useFloat(x: float): float {
    return x;
}
export function caller(): float {
    return useFloat(3.7);
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/floatarg.ts
pub fn caller() -> Result<f64, RuntimeError> {
    Ok(useFloat(3i64)?)
}
pub fn useFloat(x: f64) -> Result<f64, RuntimeError> {
    Ok(x)
}
```
Verified this is not just semantically wrong but literally does not compile as Rust:
```
$ cat /tmp/t2.rs
fn use_float(x: f64) -> f64 { x }
fn caller() -> f64 { use_float(3i64) }
fn main() {}
$ rustc --edition 2021 -o /tmp/t2out /tmp/t2.rs
error[E0308]: mismatched types
 --> /tmp/t2.rs:2:32
  |
2 | fn caller() -> f64 { use_float(3i64) }
  |                      --------- ^^^^ expected `f64`, found `i64`
```

Confirmed for array literals:
```
$ cat context/floatarr.ts
export function makeArr(): Array<float> {
    return [1.5, 2.5];
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/floatarr.ts
pub fn makeArr() -> Result<Vec<f64>, RuntimeError> {
    Ok(vec![1i64, 2i64])
}
```
`Vec<f64>` initialized with `vec![1i64, 2i64]` — same E0308 class of error.

Confirmed for `.push`:
```
$ cat context/pushfloat2.ts
export function pushIt(): Array<float> {
    let xs: Array<float> = [];
    xs.push(1.5);
    return xs;
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/pushfloat2.ts
pub fn pushIt() -> Result<Vec<f64>, RuntimeError> {
    let mut xs: Vec<f64> = vec![];
    {
        let __compylr_value = 1i64;
        (xs).push(__compylr_value);
    }
    Ok(xs)
}
```
Also `Vec<f64>::push(i64)` — same compile failure.

Also confirmed the special-cased `Math.floor` builtin (lower.rs:1350-1356, returns its argument
unchanged, no actual flooring) compounds with this: `Math.floor(3.7)` first gets its `3.7`
argument truncated to `Literal::Int(3)` by the call-argument prescan (§3 above) before
`Math.floor` ever runs, so it happens to produce `3i64` for this particular positive input, but
only by accident of two separate bugs interacting — `Math.floor(-3.7)` would need `-4` under real
JS `Math.floor` (rounds toward -infinity), and gets `-3` (`(-3.7) as i64` truncates toward zero)
even before considering that the literal-truncation bug applies before `Math.floor`'s own (missing)
logic ever sees a fractional value:
```
$ cat context/mathfloor.ts
export function floorIt(): number {
    return Math.floor(3.7);
}
$ cargo run -q -p compylr-cli -- --frontend typescript --emit rust context/mathfloor.ts
pub fn floorIt() -> Result<i64, RuntimeError> {
    Ok(3i64)
}
```

None of `frontends/typescript/fixtures/accepted/*.ts` contains a non-integral numeric literal
anywhere (checked by inspection of arithmetic.ts, branching.ts, collections.ts, loops.ts,
classes.ts) or a `float`-typed parameter/return, so this defect class has zero coverage in the
corpus that is supposed to demonstrate the subset works.
