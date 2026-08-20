//! The runtime shim, tested natively.
//!
//! `runtime.rs` has two lives: it is compiled as part of this crate, and it is embedded verbatim
//! into every generated crate via `include_str!`. Its own doc comment has always claimed the
//! first, and until the workspace split it was not true — `src/backend/mod.rs` declared
//! `bindings` and `rust` and never `runtime`, so the file compiled only inside somebody else's
//! project and its only coverage was end-to-end through a built extension.
//!
//! That is a bad place for the semantics corrections to live untested. Every helper here exists
//! because Rust's native operator is one choice among several, and the ones that disagree do so
//! only on inputs nobody reaches by accident: negative operands, `i64::MIN`, a zero divisor. The
//! tests are written from the outside for a reason — a `#[cfg(test)]` module inside `runtime.rs`
//! would be embedded into every user's `compat.rs` along with everything else.

use compylr_backend_rust::runtime::{PyAdd, PyNum, RuntimeError, div_exact};

// Called as `PyNum::div_floor(&a, &b)` rather than `a.div_floor(&b)`, which is the form the
// backend emits and the form that stays correct: std is stabilising an inherent `i64::div_floor`,
// and an inherent method wins over a trait one in method-call syntax. Emitted code has always
// been fully qualified, so generated crates are unaffected; testing the same way keeps it that
// way and keeps these tests free of a future-incompatibility warning.

mod integer_division {
    use super::*;

    #[test]
    fn flooring_and_truncation_agree_only_when_signs_do() {
        // Same operands, two declared modes, two answers. This is the disagreement the IR now
        // carries explicitly, reproduced by the two helpers.
        assert_eq!(PyNum::div_floor(&(-7i64), &2), Ok(-4));
        assert_eq!(PyNum::div_trunc(&(-7i64), &2), Ok(-3));
        assert_eq!(PyNum::div_floor(&7i64, &(-2)), Ok(-4));
        assert_eq!(PyNum::div_trunc(&7i64, &(-2)), Ok(-3));

        // Sharing a sign, or dividing exactly, and they agree.
        assert_eq!(PyNum::div_floor(&7i64, &2), Ok(3));
        assert_eq!(PyNum::div_trunc(&7i64, &2), Ok(3));
        assert_eq!(PyNum::div_floor(&(-6i64), &2), Ok(-3));
        assert_eq!(PyNum::div_trunc(&(-6i64), &2), Ok(-3));
    }

    #[test]
    fn dividing_by_zero_is_reported() {
        assert_eq!(
            PyNum::div_floor(&1i64, &0),
            Err(RuntimeError::DivisionByZero)
        );
        assert_eq!(
            PyNum::div_trunc(&1i64, &0),
            Err(RuntimeError::DivisionByZero)
        );
    }

    /// The one division whose true quotient is out of range.
    ///
    /// `i64::MIN / -1` is `i64::MAX + 1`. A language with arbitrary-precision integers widens;
    /// the honest answer for a 64-bit integer is overflow, and Rust's native `/` panics instead.
    #[test]
    fn the_single_overflowing_division_is_reported() {
        assert_eq!(
            PyNum::div_floor(&i64::MIN, &(-1)),
            Err(RuntimeError::Overflow)
        );
        assert_eq!(
            PyNum::div_trunc(&i64::MIN, &(-1)),
            Err(RuntimeError::Overflow)
        );
    }
}

mod integer_remainder {
    use super::*;

    #[test]
    fn the_two_sign_conventions_disagree_on_mixed_signs() {
        assert_eq!(PyNum::rem_floor(&(-7i64), &2), Ok(1));
        assert_eq!(PyNum::rem_trunc(&(-7i64), &2), Ok(-1));
        assert_eq!(PyNum::rem_floor(&7i64, &(-2)), Ok(-1));
        assert_eq!(PyNum::rem_trunc(&7i64, &(-2)), Ok(1));

        assert_eq!(PyNum::rem_floor(&7i64, &2), Ok(1));
        assert_eq!(PyNum::rem_trunc(&7i64, &2), Ok(1));
    }

    #[test]
    fn dividing_by_zero_is_reported() {
        assert_eq!(
            PyNum::rem_floor(&1i64, &0),
            Err(RuntimeError::DivisionByZero)
        );
        assert_eq!(
            PyNum::rem_trunc(&1i64, &0),
            Err(RuntimeError::DivisionByZero)
        );
    }

    /// `i64::MIN % -1` overflows in Rust though the answer, 0, is representable.
    #[test]
    fn the_representable_answer_is_returned_where_rust_would_trap() {
        assert_eq!(PyNum::rem_floor(&i64::MIN, &(-1)), Ok(0));
        assert_eq!(PyNum::rem_trunc(&i64::MIN, &(-1)), Ok(0));
    }

    /// Each pair must reconstruct the dividend; mixing halves must not.
    #[test]
    fn each_pair_satisfies_the_division_identity() {
        for a in [-7i64, -6, -1, 0, 1, 6, 7] {
            for b in [2i64, -2, 3, -3] {
                let floored =
                    PyNum::div_floor(&a, &b).unwrap() * b + PyNum::rem_floor(&a, &b).unwrap();
                assert_eq!(floored, a, "flooring pair, a={a} b={b}");

                let truncated =
                    PyNum::div_trunc(&a, &b).unwrap() * b + PyNum::rem_trunc(&a, &b).unwrap();
                assert_eq!(truncated, a, "truncating pair, a={a} b={b}");
            }
        }
    }
}

mod float_arithmetic {
    use super::*;

    #[test]
    fn the_modes_carry_over_to_floating_point() {
        assert_eq!(PyNum::div_floor(&(-7.0f64), &2.0), Ok(-4.0));
        assert_eq!(PyNum::div_trunc(&(-7.0f64), &2.0), Ok(-3.0));
        assert_eq!(PyNum::rem_floor(&(-7.0f64), &2.0), Ok(1.0));
        assert_eq!(PyNum::rem_trunc(&(-7.0f64), &2.0), Ok(-1.0));
    }

    /// IEEE-754 would hand back infinity, which is not what a reported failure looks like.
    #[test]
    fn dividing_a_float_by_zero_is_reported_rather_than_infinite() {
        assert_eq!(div_exact(&1.0, &0.0), Err(RuntimeError::DivisionByZero));
        assert_eq!(
            PyNum::div_floor(&(1.0f64), &0.0),
            Err(RuntimeError::DivisionByZero)
        );
        assert_eq!(
            PyNum::rem_floor(&(1.0f64), &0.0),
            Err(RuntimeError::DivisionByZero)
        );
    }

    #[test]
    fn exact_division_keeps_the_fraction() {
        assert_eq!(div_exact(&7.0, &2.0), Ok(3.5));
    }

    #[test]
    fn float_arithmetic_does_not_report_overflow() {
        // Floats saturate to infinity rather than failing, which is the target's own behaviour and
        // matches what the source languages compylr accepts do.
        assert_eq!(f64::MAX.py_mul(&2.0), Ok(f64::INFINITY));
        assert_eq!(1.0f64.py_sub(&0.5), Ok(0.5));
        assert_eq!(1.0f64.py_neg(), Ok(-1.0));
    }
}

mod checked_arithmetic {
    use super::*;

    /// Overflow is reported, not wrapped. This is a guarantee the backend declares.
    #[test]
    fn integer_overflow_is_reported_in_every_operation() {
        assert_eq!(i64::MAX.py_add(&1), Err(RuntimeError::Overflow));
        assert_eq!(i64::MIN.py_sub(&1), Err(RuntimeError::Overflow));
        assert_eq!(i64::MAX.py_mul(&2), Err(RuntimeError::Overflow));
        assert_eq!(i64::MIN.py_neg(), Err(RuntimeError::Overflow));
    }

    #[test]
    fn ordinary_arithmetic_succeeds() {
        assert_eq!(2i64.py_add(&3), Ok(5));
        assert_eq!(2i64.py_sub(&3), Ok(-1));
        assert_eq!(2i64.py_mul(&3), Ok(6));
        assert_eq!(2i64.py_neg(), Ok(-2));
        assert_eq!(1.5f64.py_add(&2.25), Ok(3.75));
    }

    /// Addition is the one arithmetic operator strings support.
    #[test]
    fn strings_concatenate() {
        assert_eq!(
            "a".to_string().py_add(&"b".to_string()),
            Ok("ab".to_string())
        );
    }
}

mod failures {
    use super::*;

    /// Every failure must render, because each becomes a message a user reads.
    #[test]
    fn every_failure_renders_distinctly() {
        let all = [
            RuntimeError::DivisionByZero,
            RuntimeError::Overflow,
            RuntimeError::IndexOutOfRange,
            RuntimeError::ZeroStep,
            RuntimeError::MissingKey("k".to_string()),
        ];
        let mut rendered: Vec<String> = all.iter().map(ToString::to_string).collect();
        assert!(rendered.iter().all(|text| !text.is_empty()));
        rendered.sort();
        let count = rendered.len();
        rendered.dedup();
        assert_eq!(rendered.len(), count, "failures must be distinguishable");
    }

    /// A missing key names the key, because that is the whole content of the diagnostic.
    #[test]
    fn a_missing_key_names_the_key() {
        assert!(
            RuntimeError::MissingKey("absent".to_string())
                .to_string()
                .contains("absent")
        );
    }
}
