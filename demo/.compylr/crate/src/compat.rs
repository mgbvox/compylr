//! Python arithmetic semantics, in Rust.
//!
//! This file has two lives. It is compiled as part of compylr, so its behavior is unit-tested
//! natively; and it is embedded verbatim into every generated crate via `include_str!`, so
//! generated code carries the same helpers without depending on compylr at build time. That is
//! why it must stay **self-contained**: no `use crate::...`, no external crates, nothing that
//! would fail to compile once pasted into somebody else's project.
//!
//! Everything here exists because Rust's native operators disagree with Python's:
//!
//! | Expression | Python | Rust native |
//! | --- | --- | --- |
//! | `-7 // 2` | `-4` (floors) | `-3` (truncates) |
//! | `-7 % 2` | `1` (sign of divisor) | `-1` (sign of dividend) |
//! | `7 / 2` | `3.5` (always float) | `3` (integer division) |
//! | `1 / 0` | `ZeroDivisionError` | panics, or `inf` for floats |
//! | overflow | promotes to big int | wraps silently in release |
//!
//! The operations are exposed as traits rather than free functions so a backend can emit
//! `PyNum::py_sub(&a, &b)?` without knowing whether `a` is an integer or a float — Rust picks the
//! implementation by type. Without that, emission would have to re-derive operand types that
//! lowering already worked out, duplicating the type checker in the backend.
//!
//! Every operation takes references and returns an owned value, so emitted code never moves a
//! `String` out of a variable that is used again later.

use std::fmt;

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
    /// Floor-divide, rounding toward negative infinity.
    fn py_floordiv(&self, rhs: &Self) -> Result<Self, RuntimeError>;
    /// Remainder, taking the sign of the divisor.
    fn py_mod(&self, rhs: &Self) -> Result<Self, RuntimeError>;
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

impl PyNum for i64 {
    fn py_sub(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        self.checked_sub(*rhs).ok_or(RuntimeError::Overflow)
    }

    fn py_mul(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        self.checked_mul(*rhs).ok_or(RuntimeError::Overflow)
    }

    /// Python floors; Rust truncates toward zero. They agree only when the division is exact or
    /// both operands share a sign, so the quotient is corrected whenever the remainder is
    /// non-zero and its sign disagrees with the divisor's.
    fn py_floordiv(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let (a, b) = (*self, *rhs);
        if b == 0 {
            return Err(RuntimeError::DivisionByZero);
        }
        // `i64::MIN / -1` is the one division that overflows: the true quotient is one past
        // `i64::MAX`. Python would widen to a big integer; the honest answer here is overflow.
        let quotient = a.checked_div(b).ok_or(RuntimeError::Overflow)?;
        let remainder = a % b;
        if remainder != 0 && ((remainder < 0) != (b < 0)) {
            // Cannot overflow: a non-zero remainder means the quotient is strictly inside range.
            Ok(quotient - 1)
        } else {
            Ok(quotient)
        }
    }

    /// Python's remainder takes the sign of the divisor; Rust's takes the sign of the dividend.
    fn py_mod(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        let (a, b) = (*self, *rhs);
        if b == 0 {
            return Err(RuntimeError::DivisionByZero);
        }
        // `i64::MIN % -1` overflows in Rust even though the answer, 0, is representable. Unlike
        // floor division there is nothing out of range here, so returning the real answer is
        // both correct and what Python gives.
        if b == -1 {
            return Ok(0);
        }
        let remainder = a % b;
        if remainder != 0 && ((remainder < 0) != (b < 0)) {
            // Cannot overflow: `|remainder| < |b|`, so the sum moves toward zero.
            Ok(remainder + b)
        } else {
            Ok(remainder)
        }
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

    fn py_floordiv(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }
        Ok((self / rhs).floor())
    }

    fn py_mod(&self, rhs: &Self) -> Result<Self, RuntimeError> {
        if *rhs == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }
        let remainder = self % rhs;
        if remainder != 0.0 && ((remainder < 0.0) != (*rhs < 0.0)) {
            Ok(remainder + rhs)
        } else {
            Ok(remainder)
        }
    }

    fn py_neg(&self) -> Result<Self, RuntimeError> {
        Ok(-self)
    }
}

/// True division, which always yields a float.
///
/// Both operands are already `f64`: lowering wraps integer operands in an explicit promotion
/// node, so the backend never has to widen them itself.
///
/// Python raises `ZeroDivisionError` for `1.0 / 0.0` where IEEE-754 would hand back infinity, so
/// the zero check applies to floats too — this is not an integer-only concern.
pub fn py_truediv(lhs: &f64, rhs: &f64) -> Result<f64, RuntimeError> {
    if *rhs == 0.0 {
        return Err(RuntimeError::DivisionByZero);
    }
    Ok(lhs / rhs)
}

/// Read a sequence element the way Python indexes.
///
/// Python counts a negative index from the end; Rust does not, and `xs[-1]` would either fail to
/// compile or wrap into an enormous positive index. Reading past either end is reported rather
/// than panicking, because Python reports it to the program.
///
/// The element is cloned out, which is what lets the read-only subset work without threading
/// borrows through generated code. For a scalar that is free.
pub fn py_index<T: Clone>(items: &[T], index: i64) -> Result<T, RuntimeError> {
    let length = items.len() as i64;
    let resolved = if index < 0 { index + length } else { index };
    if resolved < 0 || resolved >= length {
        return Err(RuntimeError::IndexOutOfRange);
    }
    Ok(items[resolved as usize].clone())
}

/// Read a mapping value, reporting a missing key the way Python does.
pub fn py_key<K, V>(map: &std::collections::HashMap<K, V>, key: &K) -> Result<V, RuntimeError>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: Clone,
{
    map.get(key)
        .cloned()
        .ok_or_else(|| RuntimeError::MissingKey(format!("{key:?}")))
}

/// The number of characters in a string.
///
/// **Not** `String::len`, which counts UTF-8 bytes. Python counts code points, so `len("é")` is 1
/// there and 2 in Rust — correct for ASCII and silently wrong for anything else, which is the
/// same class of mistake as mapping `//` onto `/`.
pub fn py_str_len(value: &str) -> i64 {
    value.chars().count() as i64
}

/// Reading one element of a collection, dispatched by the collection's type.
///
/// A trait for the same reason arithmetic is one: the IR does not annotate expressions with their
/// types, so the backend emits `py_subscript(&(c), &(i))?` and Rust selects the implementation.
/// Unlike arithmetic, the *result* type differs per container, which is what `Output` carries.
pub trait PyIndexable<I> {
    /// What reading an element yields.
    type Output;
    /// Read one element.
    fn py_get(&self, index: &I) -> Result<Self::Output, RuntimeError>;
}

impl<T: Clone> PyIndexable<i64> for Vec<T> {
    type Output = T;
    fn py_get(&self, index: &i64) -> Result<T, RuntimeError> {
        py_index(self, *index)
    }
}

impl<K, V> PyIndexable<K> for std::collections::HashMap<K, V>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
    V: Clone,
{
    type Output = V;
    fn py_get(&self, key: &K) -> Result<V, RuntimeError> {
        py_key(self, key)
    }
}

/// Read one element of a collection.
pub fn py_subscript<C, I>(collection: &C, index: &I) -> Result<C::Output, RuntimeError>
where
    C: PyIndexable<I>,
{
    collection.py_get(index)
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
        // as it is on a read. Negative indices count from the end, as they do everywhere else.
        let length = self.len() as i64;
        let resolved = if *index < 0 { index + length } else { *index };
        if resolved < 0 || resolved >= length {
            return Err(RuntimeError::IndexOutOfRange);
        }
        self[resolved as usize] = value;
        Ok(())
    }
}

impl<K, V> PySetItem<K, V> for std::collections::HashMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    fn py_set(&mut self, key: &K, value: V) -> Result<(), RuntimeError> {
        self.insert(key.clone(), value);
        Ok(())
    }
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

impl<K, V> PyContains<K> for std::collections::HashMap<K, V>
where
    K: std::hash::Hash + Eq,
{
    fn py_contains(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<T> PyContains<T> for std::collections::HashSet<T>
where
    T: std::hash::Hash + Eq,
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

/// The number of elements Python would report.
pub trait PyLen {
    /// The length.
    fn py_len(&self) -> i64;
}

impl<T> PyLen for Vec<T> {
    fn py_len(&self) -> i64 {
        self.len() as i64
    }
}

impl<K, V> PyLen for std::collections::HashMap<K, V> {
    fn py_len(&self) -> i64 {
        self.len() as i64
    }
}

impl<T> PyLen for std::collections::HashSet<T> {
    fn py_len(&self) -> i64 {
        self.len() as i64
    }
}

impl PyLen for String {
    /// Characters, not bytes — see [`py_str_len`].
    fn py_len(&self) -> i64 {
        py_str_len(self)
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
}

impl<T: Clone> PyIterate for Vec<T> {
    type Item = T;
    fn py_iter(&self) -> impl Iterator<Item = T> + '_ {
        self.iter().cloned()
    }
}

impl<K: Clone, V> PyIterate for std::collections::HashMap<K, V> {
    type Item = K;
    fn py_iter(&self) -> impl Iterator<Item = K> + '_ {
        self.keys().cloned()
    }
}

impl<T: Clone> PyIterate for std::collections::HashSet<T> {
    type Item = T;
    fn py_iter(&self) -> impl Iterator<Item = T> + '_ {
        self.iter().cloned()
    }
}
