//! The semantics the IR declares, in Rust.
//!
//! This file has two lives. It is compiled as part of this crate, so its behavior is unit-tested
//! natively — see `tests/runtime.rs`, which lives outside this file precisely so that a
//! `#[cfg(test)]` module here does not get embedded into every user's `compat.rs`; and it is
//! embedded verbatim into every generated crate via `include_str!`, so generated code carries the
//! same helpers without depending on compylr at build time. That is why it must stay
//! **self-contained**: no `use crate::...`, no external crates, nothing that would fail to
//! compile once pasted into somebody else's project.
//!
//! Everything here exists because Rust's native operators are one choice among several, and the
//! IR says which choice the source language made:
//!
//! | Declared on the node | Result for `-7 ? 2` | Rust's native operator |
//! | --- | --- | --- |
//! | division rounding toward negative infinity | `-4` | `-3` (truncates) |
//! | division rounding toward zero | `-3` | `-3` |
//! | remainder taking the sign of the divisor | `1` | `-1` (sign of dividend) |
//! | remainder taking the sign of the dividend | `-1` | `-1` |
//! | exact division | `-3.5` | `-3` (integer division) |
//!
//! Two failures are reported rather than trapped, in every mode: division by zero, which Rust
//! panics on and which yields infinity for floats, and an integer result outside `i64`. Both are
//! guarantees a frontend declares and this backend preserves — see `compylr_ir::Guarantee`.
//!
//! The container helpers take their mode too: [`IndexOrigin`] decides whether a negative offset
//! counts from the end, and [`TextUnits`] decides what a string's length counts.
//!
//! Three container behaviours are deliberately *not* modes, and it is worth saying why rather than
//! leaving a reader to wonder whether they were overlooked:
//!
//! * **A missing mapping key** is reported. Go yields the value type's zero and TypeScript yields
//!   `undefined`, but `v, ok := m[k]` is not `m[k]` with a setting — it is a different expression
//!   with a different result type, and modelling it would need a notion of a type's zero the IR
//!   does not have. A frontend that means it lowers to a different form.
//! * **Iterating a mapping** yields keys, which is what Python, Go's `range`, and TypeScript's
//!   `for...in` all do.
//! * **Membership in a string** tests substrings, which is what `in`, `strings.Contains`,
//!   `includes`, and `find` all do.
//!
//! Those keep their `Py` prefix because they are one language's reading of a question the others
//! answer the same way — a conclusion, not a gap.
//!
//! The operations are exposed as traits rather than free functions so a backend can emit
//! `PyNum::py_sub(&a, &b)?` without knowing whether `a` is an integer or a float — Rust picks the
//! implementation by type. Without that, emission would have to re-derive operand types that
//! lowering already worked out, duplicating the type checker in the backend.
//!
//! Every operation takes references and returns an owned value, so emitted code never moves a
//! `String` out of a variable that is used again later.

use std::fmt;

/// How a negative offset into a sequence is resolved.
///
/// A mirror of the IR's own enum. Duplicated rather than shared, because this file is embedded
/// verbatim into every generated crate and may not name anything outside itself — a generated crate
/// depending on compylr at build time would defeat the point of embedding it. The backend's
/// `rust_index_origin` is the seam, and a test asserts the two stay in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexOrigin {
    /// A negative offset counts backwards from the end: `xs[-1]` is the last element.
    FromEitherEnd,
    /// A negative offset is out of range.
    FromStart,
}

/// What the length of a text value counts.
///
/// A mirror of the IR's own enum, for the reason [`IndexOrigin`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextUnits {
    /// Unicode scalar values.
    CodePoints,
    /// Bytes of the UTF-8 encoding.
    Utf8Bytes,
    /// Units of the UTF-16 encoding.
    Utf16Units,
}

/// A failure inside compiled code that Python reports to the program rather than crashing on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// A division or remainder whose divisor was zero.
    DivisionByZero,
    /// A result outside the range of a 64-bit signed integer.
    Overflow,
    /// A sequence index outside its bounds, in either direction.
    IndexOutOfRange,
    /// A range whose step was zero, which would never terminate.
    ZeroStep,
    /// A key that is not present in a mapping.
    ///
    /// Carries the key rendered as text because Python's `KeyError` shows it. Making the error
    /// generic over the key type would infect every signature here for one message.
    MissingKey(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::Overflow => write!(f, "integer overflow"),
            Self::IndexOutOfRange => write!(f, "index out of range"),
            Self::ZeroStep => write!(f, "range() arg 3 must not be zero"),
            Self::MissingKey(key) => write!(f, "{key}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Addition, which Python defines for numbers *and* strings.
///
/// Split from [`PyNum`] because concatenation is the only arithmetic operator strings support;
/// putting it in the numeric trait would mean implementing subtraction for `String`.
pub trait PyAdd: Sized {
    /// Add or concatenate.
    fn py_add(&self, rhs: &Self) -> Result<Self, RuntimeError>;
}

/// Arithmetic defined only for numbers.
pub trait PyNum: Sized {
    /// Subtract.
    fn py_sub(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Multiply.
    fn py_mul(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Divide, rounding toward negative infinity: `-7 / 2` is `-4`.
    fn div_floor(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Divide, rounding toward zero: `-7 / 2` is `-3`.
    fn div_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Remainder taking the sign of the divisor: `-7 % 2` is `1`.
    ///
    /// The companion of [`Self::div_floor`]: `(a / b) * b + (a % b) == a` holds within a pair
    /// and fails across one.
    fn rem_floor(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Remainder taking the sign of the dividend: `-7 % 2` is `-1`.
    ///
    /// The companion of [`Self::div_trunc`].
    fn rem_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Negate.
    fn py_neg(&self) -> Result<Self, RuntimeError>;
}

impl PyAdd for i64 {
    fn py_add(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        self.checked_add(*rhs).ok_or(RuntimeError::Overflow)
    }
}

impl PyAdd for f64 {
    fn py_add(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        Ok(self + rhs)
    }
}

impl PyAdd for String {
    fn py_add(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let mut joined = String::with_capacity(self.len() + rhs.len());
        joined.push_str(self);
        joined.push_str(rhs);
        Ok(joined)
    }
}

/// In-place addition, for an accumulator that reads itself.
///
/// The shape `x = x + y` is ordinary Python and, for text, quadratic when emitted as a rebuild:
/// every step allocates a fresh value and copies everything accumulated so far. CPython resizes in
/// place when the target holds the only reference, so rebuilding was asymptotically *worse* than
/// the interpreter being replaced — the one case where compiling made an algorithm slower rather
/// than merely failing to make it faster.
///
/// A separate trait rather than a default method on [`PyAdd`], so each type states how it
/// accumulates and the emitter never has to know which type it is looking at. The numeric
/// implementations keep the checked arithmetic of `py_add` exactly: an in-place update that
/// stopped reporting overflow would be a change of meaning wearing the costume of an optimization.
pub trait PyAddAssign {
    /// Add or concatenate `rhs` into `self`.
    fn py_add_assign(&mut self, rhs: &Self) -> Result<(), RuntimeError>;
}

impl PyAddAssign for i64 {
    /// Still checked. `py_add` reports overflow and so does this.
    fn py_add_assign(&mut self, rhs: &Self) -> Result<(), RuntimeError> {
        *self = self.checked_add(*rhs).ok_or(RuntimeError::Overflow)?;
        Ok(())
    }
}

impl PyAddAssign for f64 {
    fn py_add_assign(&mut self, rhs: &Self) -> Result<(), RuntimeError> {
        *self += *rhs;
        Ok(())
    }
}

impl PyAddAssign for String {
    /// The reason the trait exists: append, rather than allocate a new string and copy both
    /// halves into it.
    fn py_add_assign(&mut self, rhs: &Self) -> Result<(), RuntimeError> {
        self.push_str(rhs);
        Ok(())
    }
}

impl PyNum for i64 {
    fn py_sub(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        self.checked_sub(*rhs).ok_or(RuntimeError::Overflow)
    }

    fn py_mul(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        self.checked_mul(*rhs).ok_or(RuntimeError::Overflow)
    }

    /// Rust's `/` truncates. Flooring agrees with it only when the division is exact or both
    /// operands share a sign, so the quotient is corrected whenever the remainder is non-zero and
    /// its sign disagrees with the divisor's.
    fn div_floor(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let (a, b) = (*self, *rhs);
        let quotient = self.div_trunc(rhs)?;
        let remainder = a % b;
        if remainder != 0 && ((remainder < 0) != (b < 0)) {
            // Cannot overflow: a non-zero remainder means the quotient is strictly inside range.
            Ok(quotient - 1)
        } else {
            Ok(quotient)
        }
    }

    /// This is Rust's own `/`, with the two failures reported rather than trapped.
    fn div_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0 {
            return Err(RuntimeError::DivisionByZero);
        }
        // `i64::MIN / -1` is the one division that overflows: the true quotient is one past
        // `i64::MAX`. A language with arbitrary-precision integers would widen; the honest answer
        // for a 64-bit integer is overflow.
        self.checked_div(*rhs).ok_or(RuntimeError::Overflow)
    }

    fn rem_floor(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let (a, b) = (*self, *rhs);
        let remainder = self.rem_trunc(rhs)?;
        if remainder != 0 && ((remainder < 0) != (b < 0)) {
            // Cannot overflow: `|remainder| < |b|`, so the sum moves toward zero.
            let _ = a;
            Ok(remainder + b)
        } else {
            Ok(remainder)
        }
    }

    /// This is Rust's own `%`, with the two failures reported rather than trapped.
    fn rem_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let (a, b) = (*self, *rhs);
        if b == 0 {
            return Err(RuntimeError::DivisionByZero);
        }
        // `i64::MIN % -1` overflows in Rust even though the answer, 0, is representable. Unlike
        // division there is nothing out of range here, so returning the real answer is correct.
        if b == -1 {
            return Ok(0);
        }
        Ok(a % b)
    }

    fn py_neg(&self) -> Result<Self, RuntimeError> {
        self.checked_neg().ok_or(RuntimeError::Overflow)
    }
}

impl PyNum for f64 {
    fn py_sub(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        Ok(self - rhs)
    }

    fn py_mul(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        Ok(self * rhs)
    }

    fn div_floor(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }
        Ok((self / rhs).floor())
    }

    fn div_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }
        Ok((self / rhs).trunc())
    }

    fn rem_floor(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let remainder = self.rem_trunc(rhs)?;
        if remainder != 0.0 && ((remainder < 0.0) != (*rhs < 0.0)) {
            Ok(remainder + rhs)
        } else {
            Ok(remainder)
        }
    }

    fn rem_trunc(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }
        Ok(self % rhs)
    }

    fn py_neg(&self) -> Result<Self, RuntimeError> {
        Ok(-self)
    }
}

/// Exact division, which always yields a float.
///
/// Both operands are already `f64`: lowering wraps integer operands in an explicit promotion
/// node, so the backend never has to widen them itself.
///
/// The zero check applies to floats too, where IEEE-754 would hand back infinity. Reporting it
/// is what the [`RuntimeError::DivisionByZero`] guarantee means, and a frontend whose language
/// wants infinity would declare a different mode rather than reach this function.
pub fn div_exact(lhs: &f64, rhs: &f64) -> Result<f64, RuntimeError> {
    if *rhs == 0.0 {
        return Err(RuntimeError::DivisionByZero);
    }
    Ok(lhs / rhs)
}

/// Read a sequence element, resolving a negative offset the way the node declared.
///
/// Under [`IndexOrigin::FromEitherEnd`] a negative index counts from the end, which is Python's
/// reading; under [`IndexOrigin::FromStart`] it is out of range, which is everyone else's. Reading
/// past either end is reported rather than panicking, under both.
///
/// The element is cloned out, which is what lets the read-only subset work without threading
/// borrows through generated code. For a scalar that is free.
pub fn py_index<T: Clone>(items: &[T], index: i64, origin: IndexOrigin) -> Result<T, RuntimeError> {
    items
        .get(resolve_index(index, items.len(), origin)?)
        .cloned()
        .ok_or(RuntimeError::IndexOutOfRange)
}

/// Turn a declared index into an offset, reporting only what the lookup cannot.
///
/// Shared by every sequence operation that takes an index — reading, assigning, and borrowing a
/// place to write through. Three copies of this was two too many: they are the same rule, and the
/// one that drifted would disagree with the others on exactly the inputs (a negative index, an
/// index one past the end) that a test suite is least likely to cover.
///
/// **The upper bound is deliberately not checked here.** Every caller follows this with a lookup
/// that has to test it anyway — `get`, `get_mut` — so checking it here made every element read
/// test the same bound twice and carry a panic path for a case that had just been ruled out.
/// What is left is the part a lookup genuinely cannot do: turn a negative index into an offset
/// under the origin the frontend declared, and reject one that is still negative.
///
/// `saturating_add` rather than `+`: `i64::MIN + length` overflows, which is a panic in a debug
/// build for an index that is simply out of range. Saturating leaves it enormously negative,
/// which is rejected on the next line.
fn resolve_index(index: i64, length: usize, origin: IndexOrigin) -> Result<usize, RuntimeError> {
    let resolved = match origin {
        IndexOrigin::FromEitherEnd if index < 0 => index.saturating_add(length as i64),
        // Left as it is, so it fails the range check rather than wrapping into an enormous
        // positive index — which is what a target's native indexing would do with it.
        IndexOrigin::FromEitherEnd | IndexOrigin::FromStart => index,
    };
    if resolved < 0 {
        return Err(RuntimeError::IndexOutOfRange);
    }
    // Still `usize`-sized on a 32-bit target, where a huge positive index would truncate. The
    // lookup rejects it either way, but truncating first could make it land *inside* the
    // collection, which would be a wrong answer rather than a slow one.
    usize::try_from(resolved).map_err(|_| RuntimeError::IndexOutOfRange)
}

/// The hasher generated containers use, and the reason it is not the standard one.
///
/// `RandomState` is chosen by the standard library to be resistant to a caller who is *trying* to
/// collide keys — a real concern for a web server keying a map on request headers, and a cost
/// paid by every program that hashes anything. It is also seeded per process, which is why
/// mapping iteration order already varies between runs.
///
/// The comparison this project is measured against does not pay that cost: CPython hashes a small
/// integer to itself, and caches a string's hash inside the string object. Generated code was
/// paying SipHash per lookup against an interpreter paying nearly nothing, which is most of why
/// `graphs.bfs_distances` ran slower compiled than interpreted.
///
/// **A hasher has no observable semantics.** It changes no answer, and mapping and set iteration
/// order is already unguaranteed and already varies between runs, so a program this could break
/// was broken before. That is exactly what makes it a performance choice rather than a behavior
/// axis: `add-behavior-profiles` is built on axes where two languages disagree about *meaning*,
/// and here nothing disagrees — one option is simply faster.
///
/// **What is given up:** this is not a cryptographic hash and offers no resistance to deliberate
/// collisions. A compiled function that builds a mapping keyed by values an attacker chooses can
/// be driven quadratic. That is a real trade and it is made deliberately, because the keys in the
/// supported subset come from the user's own program. A program keying on untrusted input should
/// not be relying on a transpiler's default hasher for its safety in any case.
///
/// The algorithm is the multiply-xor-rotate construction rustc uses for its own internal maps.
/// Written out here rather than depended on, because this file is embedded verbatim into every
/// generated crate and must compile with no dependency but `std`.
const FAST_HASH_SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

/// A fast, non-cryptographic hasher. See [`FAST_HASH_SEED`] for the trade being made.
#[derive(Default, Clone, Copy, Debug)]
pub struct FastHasher {
    hash: u64,
}

impl FastHasher {
    /// Fold one word into the accumulated hash.
    ///
    /// The rotate is what keeps the high bits from being the only ones that move: multiplication
    /// alone propagates upward, so consecutive small integers would otherwise differ only near
    /// the top of the word and collide in a table that indexes off the bottom.
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(FAST_HASH_SEED);
    }
}

impl std::hash::Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while let Some((chunk, tail)) = rest.split_first_chunk::<8>() {
            self.add(u64::from_le_bytes(*chunk));
            rest = tail;
        }
        if let Some((chunk, tail)) = rest.split_first_chunk::<4>() {
            self.add(u32::from_le_bytes(*chunk) as u64);
            rest = tail;
        }
        for &byte in rest {
            self.add(byte as u64);
        }
    }

    // Each of these would otherwise go through `write` and its byte loop. Integers are the
    // overwhelmingly common key in generated code, so the whole point is that they do not.
    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.add(value as u64);
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    #[inline]
    fn write_u128(&mut self, value: u128) {
        self.add(value as u64);
        self.add((value >> 64) as u64);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
}

/// Builds [`FastHasher`]s. Unseeded, because there is no seed to keep secret.
#[derive(Default, Clone, Copy, Debug)]
pub struct FastHashBuilder;

impl std::hash::BuildHasher for FastHashBuilder {
    type Hasher = FastHasher;

    #[inline]
    fn build_hasher(&self) -> FastHasher {
        FastHasher::default()
    }
}

/// The mapping type generated code uses.
///
/// An alias rather than a new type: it *is* a `HashMap`, so every implementation above applies to
/// it and a reader meets a familiar type rather than a wrapper to learn.
pub type FastMap<K, V> = std::collections::HashMap<K, V, FastHashBuilder>;

/// The set type generated code uses. See [`FastMap`].
pub type FastSet<T> = std::collections::HashSet<T, FastHashBuilder>;

/// Read a mapping value, reporting a missing key the way Python does.
pub fn py_key<K, V, S>(map: &std::collections::HashMap<K, V, S>, key: &K) -> Result<V, RuntimeError>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: Clone,
    S: std::hash::BuildHasher,
{
    map.get(key)
        .cloned()
        .ok_or_else(|| RuntimeError::MissingKey(format!("{key:?}")))
}

/// The length of a string, in the units the node declared.
///
/// The three readings agree on ASCII and disagree on everything else, which is what makes assuming
/// one of them survive most tests — the same class of mistake as mapping `//` onto `/`. Rust's own
/// `String::len` is the UTF-8 byte count, so it is the *only* one of the three that comes for free.
pub fn py_str_len(value: &str, units: TextUnits) -> i64 {
    match units {
        // The shortcut is **exact**, not approximate: in ASCII every byte is one code point and
        // one UTF-16 unit, so the byte count is the answer rather than an estimate of it. It
        // matters because `chars().count()` decodes the whole string on every `len()`, and
        // `is_ascii` is a vectorized scan that the common case passes.
        TextUnits::CodePoints | TextUnits::Utf16Units if value.is_ascii() => value.len() as i64,
        TextUnits::CodePoints => value.chars().count() as i64,
        TextUnits::Utf8Bytes => value.len() as i64,
        TextUnits::Utf16Units => value.chars().map(char::len_utf16).sum::<usize>() as i64,
    }
}

/// Reading one element of a collection, dispatched by the collection's type.
///
/// A trait for the same reason arithmetic is one: the IR does not annotate expressions with their
/// types, so the backend emits one call and Rust selects the implementation. Unlike arithmetic, the
/// *result* type differs per container, which is what `Output` carries.
///
/// The origin reaches every implementation, including the ones it means nothing to. A mapping has
/// no ends to count from, so its implementation ignores the argument — which is the cost of
/// carrying the mode on a node that covers both container kinds.
pub trait PyIndexable<I> {
    /// What reading an element yields.
    type Output;
    /// Read one element.
    fn py_get(&self, index: &I, origin: IndexOrigin) -> Result<Self::Output, RuntimeError>;
}

impl<T: Clone> PyIndexable<i64> for Vec<T> {
    type Output = T;
    fn py_get(&self, index: &i64, origin: IndexOrigin) -> Result<T, RuntimeError> {
        py_index(self, *index, origin)
    }
}

impl<K, V, S> PyIndexable<K> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: Clone,
    S: std::hash::BuildHasher,
{
    type Output = V;
    /// A key is not an offset, so there is nothing for the origin to decide.
    fn py_get(&self, key: &K, _origin: IndexOrigin) -> Result<V, RuntimeError> {
        py_key(self, key)
    }
}

/// Read one element of a collection.
pub fn py_subscript<C, I>(
    collection: &C,
    index: &I,
    origin: IndexOrigin,
) -> Result<C::Output, RuntimeError>
where
    C: PyIndexable<I>,
{
    collection.py_get(index, origin)
}

/// Assigning to one element of a collection.
///
/// Deliberately **not** routed through [`PyIndexable`]. Reading a missing key is a `KeyError`;
/// assigning to one creates it. They share a spelling in Python and are different operations, and
/// conflating them would either make reads silently create entries or make assignments fail on any
/// key that was not already there.
pub trait PySetItem<I, V> {
    /// Assign one element, creating it where the container's semantics say to.
    fn py_set(&mut self, index: &I, value: V) -> Result<(), RuntimeError>;
}

impl<T> PySetItem<i64, T> for Vec<T> {
    fn py_set(&mut self, index: &i64, value: T) -> Result<(), RuntimeError> {
        // A sequence has no element to create, so an out-of-range index is an error here exactly
        // as it is on a read.
        //
        // `FromEitherEnd` is passed rather than read off the node: `Stmt::SetItem` carries no
        // origin, so assignment resolves a negative index the way Python does whatever the
        // frontend declared for *reads*. That is right for every frontend in this repository and
        // is a gap for one that declares `FromStart` — recorded here rather than papered over,
        // because closing it is an IR change and not a runtime one.
        let resolved = resolve_index(*index, self.len(), IndexOrigin::FromEitherEnd)?;
        *self
            .get_mut(resolved)
            .ok_or(RuntimeError::IndexOutOfRange)? = value;
        Ok(())
    }
}

impl<K, V, S> PySetItem<K, V> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq + Clone,
    S: std::hash::BuildHasher,
{
    fn py_set(&mut self, key: &K, value: V) -> Result<(), RuntimeError> {
        self.insert(key.clone(), value);
        Ok(())
    }
}

/// Borrowing one element of a collection so that it can be read *through*.
///
/// The shared counterpart to [`PyPlace`], and the reason both exist: [`py_subscript`] hands back
/// a **clone**, which is right for the value a program asked for and wrong for an intermediate it
/// only passes through. `m[i][j]` cloned the whole row `m[i]` to read one element of it, so a
/// matrix multiply allocated and copied a row per element access — O(n^4) work for an O(n^3)
/// algorithm, with every answer correct. Nothing but a benchmark could find that.
pub trait PyBorrow<I> {
    /// What one element is.
    type Item;
    /// Borrow one element.
    fn py_borrow(&self, index: &I, origin: IndexOrigin) -> Result<&Self::Item, RuntimeError>;
}

impl<T> PyBorrow<i64> for Vec<T> {
    type Item = T;
    fn py_borrow(&self, index: &i64, origin: IndexOrigin) -> Result<&T, RuntimeError> {
        self.get(resolve_index(*index, self.len(), origin)?)
            .ok_or(RuntimeError::IndexOutOfRange)
    }
}

impl<K, V, S> PyBorrow<K> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    S: std::hash::BuildHasher,
{
    type Item = V;
    /// A key is not an offset, so there is nothing for the origin to decide. A missing key reports
    /// exactly as [`py_key`] does — borrowing through one is still reading it.
    fn py_borrow(&self, key: &K, _origin: IndexOrigin) -> Result<&V, RuntimeError> {
        self.get(key)
            .ok_or_else(|| RuntimeError::MissingKey(format!("{key:?}")))
    }
}

/// Borrow one element of a collection, for reading through.
pub fn py_borrow<'a, C, I>(
    collection: &'a C,
    index: &I,
    origin: IndexOrigin,
) -> Result<&'a C::Item, RuntimeError>
where
    C: PyBorrow<I>,
{
    collection.py_borrow(index, origin)
}

/// Borrowing one element of a collection so that it can be written *through*.
///
/// The counterpart to [`PySetItem`] for a **nested** target. `table[i][j] = v` assigns into the
/// row `table[i]`, and reaching that row with [`py_subscript`] would hand back a clone of it: the
/// assignment compiles, runs, and is lost. A place is a borrow rather than a value, so the write
/// lands where the program said it would.
///
/// The failure this prevents is silent — no error, no wrong-looking code, just a table that comes
/// back holding whatever it was initialised with. `execution.rs` runs it rather than reading it
/// for that reason.
pub trait PyPlace<I> {
    /// What one element is.
    type Item;
    /// Borrow one element mutably.
    fn py_place(&mut self, index: &I, origin: IndexOrigin)
    -> Result<&mut Self::Item, RuntimeError>;
}

impl<T> PyPlace<i64> for Vec<T> {
    type Item = T;
    fn py_place(&mut self, index: &i64, origin: IndexOrigin) -> Result<&mut T, RuntimeError> {
        let resolved = resolve_index(*index, self.len(), origin)?;
        self.get_mut(resolved).ok_or(RuntimeError::IndexOutOfRange)
    }
}

impl<K, V, S> PyPlace<K> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    S: std::hash::BuildHasher,
{
    type Item = V;
    /// A key is not an offset, so there is nothing for the origin to decide.
    ///
    /// A missing key **reports**, exactly as reading one does. `d[k][0] = v` needs `d[k]` to
    /// already exist — inserting an empty container here would invent a value the program never
    /// wrote, and would then succeed at storing something into it.
    fn py_place(&mut self, key: &K, _origin: IndexOrigin) -> Result<&mut V, RuntimeError> {
        self.get_mut(key)
            .ok_or_else(|| RuntimeError::MissingKey(format!("{key:?}")))
    }
}

/// Borrow one element of a collection, for writing through.
pub fn py_place<'a, C, I>(
    collection: &'a mut C,
    index: &I,
    origin: IndexOrigin,
) -> Result<&'a mut C::Item, RuntimeError>
where
    C: PyPlace<I>,
{
    collection.py_place(index, origin)
}

/// Membership, meaning whatever the container means by it.
///
/// A trait for the same reason subscripting is one: the IR does not annotate expressions with their
/// types, so the backend emits one call and Rust selects the implementation rather than the backend
/// re-deriving what the type checker already knew.
///
/// Two of these are **not** what a naive containment check would do, and both match Python: a
/// mapping tests its **keys**, and a string tests **substrings** rather than characters.
pub trait PyContains<T> {
    /// Whether the value is present.
    fn py_contains(&self, value: &T) -> bool;
}

impl<T: PartialEq> PyContains<T> for Vec<T> {
    fn py_contains(&self, value: &T) -> bool {
        self.contains(value)
    }
}

impl<K, V, S> PyContains<K> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq,
    S: std::hash::BuildHasher,
{
    fn py_contains(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<T, S> PyContains<T> for std::collections::HashSet<T, S>
where
    T: std::hash::Hash + Eq,
    S: std::hash::BuildHasher,
{
    fn py_contains(&self, value: &T) -> bool {
        self.contains(value)
    }
}

impl PyContains<String> for String {
    fn py_contains(&self, value: &String) -> bool {
        // A substring test. `"ab" in "cab"` is true in Python, and a character-membership reading
        // would answer false.
        self.contains(value.as_str())
    }
}

/// The length of a value, in the units the node declared.
///
/// The units reach every implementation, and only text has more than one answer to give. A
/// collection's length is a count of its elements under every reading, so those implementations
/// ignore the argument.
pub trait PyLen {
    /// The length.
    fn py_len(&self, units: TextUnits) -> i64;
}

impl<T> PyLen for Vec<T> {
    fn py_len(&self, _units: TextUnits) -> i64 {
        self.len() as i64
    }
}

impl<K, V, S> PyLen for std::collections::HashMap<K, V, S> {
    fn py_len(&self, _units: TextUnits) -> i64 {
        self.len() as i64
    }
}

impl<T, S> PyLen for std::collections::HashSet<T, S> {
    fn py_len(&self, _units: TextUnits) -> i64 {
        self.len() as i64
    }
}

impl PyLen for String {
    /// The one implementation the units decide — see [`py_str_len`].
    fn py_len(&self, units: TextUnits) -> i64 {
        py_str_len(self, units)
    }
}

/// Iterating a collection the way Python does.
///
/// A trait for the same reason subscripting is one: the backend emits one call and Rust selects the
/// implementation by type. A mapping yields its **keys**, matching Python — which a naive
/// implementation over a map would not do.
pub trait PyIterate {
    /// What each step yields.
    type Item;
    /// The values, in whatever order the container defines.
    fn py_iter(&self) -> impl Iterator<Item = Self::Item> + '_;
    /// The same values, borrowed rather than copied.
    ///
    /// A `for` whose body only *reads* the loop variable has no use for its own copy, and for a
    /// collection of owned values that copy is an allocation per element per loop. The copying
    /// form stays, because a body that assigns to the loop variable genuinely needs one.
    fn py_iter_borrowed(&self) -> impl Iterator<Item = &Self::Item> + '_;
}

impl<T: Clone> PyIterate for Vec<T> {
    type Item = T;
    fn py_iter_borrowed(&self) -> impl Iterator<Item = &T> + '_ {
        self.iter()
    }
    fn py_iter(&self) -> impl Iterator<Item = T> + '_ {
        self.iter().cloned()
    }
}

impl<K: Clone, V, S> PyIterate for std::collections::HashMap<K, V, S> {
    type Item = K;
    fn py_iter_borrowed(&self) -> impl Iterator<Item = &K> + '_ {
        self.keys()
    }
    fn py_iter(&self) -> impl Iterator<Item = K> + '_ {
        self.keys().cloned()
    }
}

impl<T: Clone, S> PyIterate for std::collections::HashSet<T, S> {
    type Item = T;
    fn py_iter_borrowed(&self) -> impl Iterator<Item = &T> + '_ {
        self.iter()
    }
    fn py_iter(&self) -> impl Iterator<Item = T> + '_ {
        self.iter().cloned()
    }
}

// Borrowed values satisfy the reading traits, delegating to the owned implementations.
//
// A `for` whose body only reads its loop variable binds it by reference, which saves an
// allocation and a copy per element per loop. Without these, that emitter change does not
// compile: the traits are implemented on the owned types, so a borrowed loop variable does not
// satisfy them. The emitter change is a few lines; this is the work that makes it legal.
//
// Only the *reading* traits get one. `PyAdd` and `PyNum` return `Self`, and `Self` would be a
// reference here — there is no owned value to return, so a borrowed operand still has to be
// cloned into one at the point of use. That is unchanged behaviour, not a gap.

impl<T> PyLen for &T
where
    T: PyLen + ?Sized,
{
    fn py_len(&self, units: TextUnits) -> i64 {
        (**self).py_len(units)
    }
}

impl<T, I> PyContains<I> for &T
where
    T: PyContains<I> + ?Sized,
{
    fn py_contains(&self, value: &I) -> bool {
        (**self).py_contains(value)
    }
}

impl<T, I> PyIndexable<I> for &T
where
    T: PyIndexable<I> + ?Sized,
{
    type Output = T::Output;
    fn py_get(&self, index: &I, origin: IndexOrigin) -> Result<Self::Output, RuntimeError> {
        (**self).py_get(index, origin)
    }
}

impl<T, I> PyBorrow<I> for &T
where
    T: PyBorrow<I> + ?Sized,
{
    type Item = T::Item;
    fn py_borrow(&self, index: &I, origin: IndexOrigin) -> Result<&Self::Item, RuntimeError> {
        (**self).py_borrow(index, origin)
    }
}

impl<T> PyIterate for &T
where
    T: PyIterate + ?Sized,
{
    type Item = T::Item;
    fn py_iter(&self) -> impl Iterator<Item = Self::Item> + '_ {
        (**self).py_iter()
    }
    fn py_iter_borrowed(&self) -> impl Iterator<Item = &Self::Item> + '_ {
        (**self).py_iter_borrowed()
    }
}

/// Add into an indexed slot, resolving the index **once**.
///
/// `d[k] = d[k] + 1` is ordinary Python and was emitted as a read followed by a write: two
/// separate lookups of the same key, so two hashes, on a statement whose whole purpose is to
/// touch one entry. Counting occurrences — the single most common thing anyone does with a
/// mapping — paid for it once per element.
///
/// The trait is indexed rather than mapping-specific because the emitter cannot tell a mapping
/// from a sequence: it does not know an expression's type, so it emits one call and Rust selects
/// the implementation, exactly as every other container operation here works.
///
/// A missing key still **reports** and is still not created. The fused form reaches for an
/// existing slot and fails when there is none, which is what reading one already did — assignment
/// is what creates a key, and this is not an assignment.
pub trait PyAddAssignAt<I> {
    /// What one slot holds.
    type Value;
    /// Add `delta` into the slot at `index`.
    fn py_add_assign_at(
        &mut self,
        index: &I,
        delta: &Self::Value,
        origin: IndexOrigin,
    ) -> Result<(), RuntimeError>;
}

impl<T> PyAddAssignAt<i64> for Vec<T>
where
    T: PyAddAssign,
{
    type Value = T;
    fn py_add_assign_at(
        &mut self,
        index: &i64,
        delta: &T,
        origin: IndexOrigin,
    ) -> Result<(), RuntimeError> {
        let resolved = resolve_index(*index, self.len(), origin)?;
        self.get_mut(resolved)
            .ok_or(RuntimeError::IndexOutOfRange)?
            .py_add_assign(delta)
    }
}

impl<K, V, S> PyAddAssignAt<K> for std::collections::HashMap<K, V, S>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: PyAddAssign,
    S: std::hash::BuildHasher,
{
    type Value = V;
    /// A key is not an offset, so there is nothing for the origin to decide.
    fn py_add_assign_at(
        &mut self,
        key: &K,
        delta: &V,
        _origin: IndexOrigin,
    ) -> Result<(), RuntimeError> {
        self.get_mut(key)
            .ok_or_else(|| RuntimeError::MissingKey(format!("{key:?}")))?
            .py_add_assign(delta)
    }
}

impl<V, S, Q> PyAddAssignAt<&Q> for std::collections::HashMap<String, V, S>
where
    V: PyAddAssign,
    S: std::hash::BuildHasher,
    Q: AsRef<str> + ?Sized,
{
    type Value = V;
    fn py_add_assign_at(
        &mut self,
        key: &&Q,
        delta: &V,
        _origin: IndexOrigin,
    ) -> Result<(), RuntimeError> {
        self.get_mut((*key).as_ref())
            .ok_or_else(|| RuntimeError::MissingKey(format!("{:?}", (*key).as_ref())))?
            .py_add_assign(delta)
    }
}
