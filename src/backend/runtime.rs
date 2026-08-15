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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    /// A division or remainder whose divisor was zero.
    DivisionByZero,
    /// A result outside the range of a 64-bit signed integer.
    Overflow,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::Overflow => write!(f, "integer overflow"),
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
