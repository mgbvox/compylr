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

mod sequence_indexing {
    use super::*;
    use compylr_backend_rust::runtime::{IndexOrigin, PyIndexable, py_index, py_subscript};

    fn items() -> Vec<i64> {
        vec![10, 20, 30]
    }

    #[test]
    fn the_two_origins_disagree_only_on_a_negative_index() {
        assert_eq!(py_index(&items(), -1, IndexOrigin::FromEitherEnd), Ok(30));
        assert_eq!(
            py_index(&items(), -1, IndexOrigin::FromStart),
            Err(RuntimeError::IndexOutOfRange)
        );

        // Non-negative offsets are the same read under either declaration.
        for origin in [IndexOrigin::FromEitherEnd, IndexOrigin::FromStart] {
            assert_eq!(py_index(&items(), 0, origin), Ok(10), "{origin:?}");
            assert_eq!(py_index(&items(), 2, origin), Ok(30), "{origin:?}");
        }
    }

    #[test]
    fn counting_from_the_end_reaches_the_first_element_and_no_further() {
        assert_eq!(py_index(&items(), -3, IndexOrigin::FromEitherEnd), Ok(10));
        assert_eq!(
            py_index(&items(), -4, IndexOrigin::FromEitherEnd),
            Err(RuntimeError::IndexOutOfRange)
        );
    }

    /// The boundary at exactly the length, which is the off-by-one a hand-written check gets wrong.
    #[test]
    fn an_index_equal_to_the_length_is_out_of_range() {
        for origin in [IndexOrigin::FromEitherEnd, IndexOrigin::FromStart] {
            assert_eq!(
                py_index(&items(), 3, origin),
                Err(RuntimeError::IndexOutOfRange),
                "{origin:?}"
            );
        }
    }

    #[test]
    fn an_empty_sequence_has_no_readable_index() {
        let empty: Vec<i64> = vec![];
        for origin in [IndexOrigin::FromEitherEnd, IndexOrigin::FromStart] {
            assert_eq!(
                py_index(&empty, 0, origin),
                Err(RuntimeError::IndexOutOfRange)
            );
            assert_eq!(
                py_index(&empty, -1, origin),
                Err(RuntimeError::IndexOutOfRange)
            );
        }
    }

    /// A negative index must not wrap into an enormous positive one under `FromStart`.
    ///
    /// That is what a target's native indexing does with it, and it would read arbitrary memory or
    /// panic rather than report a range failure.
    #[test]
    fn a_negative_index_from_the_start_is_a_range_failure_not_a_wrap() {
        assert_eq!(
            py_index(&items(), i64::MIN, IndexOrigin::FromStart),
            Err(RuntimeError::IndexOutOfRange)
        );
    }

    #[test]
    fn the_dispatching_entry_point_carries_the_origin_through() {
        // What the backend actually emits, rather than the helper underneath it.
        assert_eq!(
            py_subscript(&items(), &-1, IndexOrigin::FromEitherEnd),
            Ok(30)
        );
        assert_eq!(
            py_subscript(&items(), &-1, IndexOrigin::FromStart),
            Err(RuntimeError::IndexOutOfRange)
        );
    }

    /// A mapping ignores the origin, because a key is not an offset.
    #[test]
    fn a_mapping_read_is_unaffected_by_the_origin() {
        let mut map = std::collections::HashMap::new();
        map.insert("k".to_string(), 7i64);

        for origin in [IndexOrigin::FromEitherEnd, IndexOrigin::FromStart] {
            assert_eq!(py_subscript(&map, &"k".to_string(), origin), Ok(7));
            assert_eq!(map.py_get(&"k".to_string(), origin), Ok(7));
        }
    }
}

mod writing_through_a_place {
    use compylr_backend_rust::runtime::{IndexOrigin, PyPlace, RuntimeError};

    // A place is a borrow, and the whole reason it exists is that the read helpers hand back a
    // clone. These assert the borrow reaches the original -- which is the property, and the one a
    // test that only checked the returned value would miss entirely.

    #[test]
    fn a_sequence_place_writes_into_the_original() {
        let mut rows = vec![vec![0i64, 0], vec![0, 0]];
        *PyPlace::py_place(&mut rows, &1i64, IndexOrigin::FromEitherEnd).unwrap() = vec![7, 8];
        assert_eq!(rows, vec![vec![0, 0], vec![7, 8]]);
    }

    #[test]
    fn a_sequence_place_resolves_the_index_the_declared_way() {
        let mut items = vec![1i64, 2, 3];
        *PyPlace::py_place(&mut items, &(-1i64), IndexOrigin::FromEitherEnd).unwrap() = 9;
        assert_eq!(items, vec![1, 2, 9], "a negative index counts from the end");

        // Under `FromStart` the same index is simply out of range, rather than counting back --
        // the reading helper has always said so, and a place that disagreed would let a frontend
        // write where it could not read.
        assert_eq!(
            PyPlace::py_place(&mut items, &(-1i64), IndexOrigin::FromStart).err(),
            Some(RuntimeError::IndexOutOfRange)
        );
    }

    #[test]
    fn a_sequence_place_past_either_end_reports() {
        let mut items = vec![1i64];
        assert_eq!(
            PyPlace::py_place(&mut items, &5i64, IndexOrigin::FromEitherEnd).err(),
            Some(RuntimeError::IndexOutOfRange)
        );
        assert_eq!(
            PyPlace::py_place(&mut items, &(-2i64), IndexOrigin::FromEitherEnd).err(),
            Some(RuntimeError::IndexOutOfRange)
        );
    }

    #[test]
    fn a_mapping_place_writes_into_the_entry_that_is_there() {
        let mut map = std::collections::HashMap::from([(String::from("k"), vec![0i64])]);
        PyPlace::py_place(&mut map, &String::from("k"), IndexOrigin::FromEitherEnd)
            .unwrap()
            .push(1);
        assert_eq!(map[&String::from("k")], vec![0, 1]);
    }

    #[test]
    fn a_mapping_place_for_a_missing_key_reports_rather_than_creating_one() {
        // The asymmetry with `PySetItem` is deliberate. `d[k] = v` creates the key; `d[k][0] = v`
        // needs it to be there already, because inserting an empty container would invent a value
        // the program never wrote and then quietly succeed at storing into it.
        let mut map: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
        let failure = PyPlace::py_place(&mut map, &String::from("absent"), IndexOrigin::FromStart);
        assert!(
            matches!(failure.err(), Some(RuntimeError::MissingKey(key)) if key.contains("absent"))
        );
        assert!(
            map.is_empty(),
            "a failed place must not have inserted anything"
        );
    }
}

mod text_length {
    use compylr_backend_rust::runtime::{PyLen, TextUnits, py_str_len};

    /// A two-byte character separates code points from bytes, and not from UTF-16 units.
    #[test]
    fn a_two_byte_character_distinguishes_two_of_the_three() {
        assert_eq!(py_str_len("é", TextUnits::CodePoints), 1);
        assert_eq!(py_str_len("é", TextUnits::Utf8Bytes), 2);
        assert_eq!(py_str_len("é", TextUnits::Utf16Units), 1);
    }

    /// A character outside the basic plane is the only input that separates all three.
    ///
    /// Without it a test could pass with UTF-16 units and code points confused, which is exactly
    /// the mistake a Python author would make writing a TypeScript backend.
    #[test]
    fn a_character_outside_the_basic_plane_distinguishes_all_three() {
        let emoji = "🦀";
        assert_eq!(py_str_len(emoji, TextUnits::CodePoints), 1);
        assert_eq!(py_str_len(emoji, TextUnits::Utf8Bytes), 4);
        assert_eq!(py_str_len(emoji, TextUnits::Utf16Units), 2);
    }

    #[test]
    fn ascii_agrees_under_every_reading() {
        // Which is why assuming one of them survives most tests.
        for units in [
            TextUnits::CodePoints,
            TextUnits::Utf8Bytes,
            TextUnits::Utf16Units,
        ] {
            assert_eq!(py_str_len("abc", units), 3, "{units:?}");
            assert_eq!(py_str_len("", units), 0, "{units:?}");
        }
    }

    #[test]
    fn a_string_measures_through_the_dispatching_trait() {
        let text = "é🦀".to_string();
        assert_eq!(PyLen::py_len(&text, TextUnits::CodePoints), 2);
        assert_eq!(PyLen::py_len(&text, TextUnits::Utf8Bytes), 6);
        assert_eq!(PyLen::py_len(&text, TextUnits::Utf16Units), 3);
    }

    /// A collection counts elements under every reading, so the units mean nothing to it.
    #[test]
    fn a_collection_ignores_the_declared_units() {
        let list = vec![1i64, 2, 3];
        let mut map = std::collections::HashMap::new();
        map.insert(1i64, 2i64);
        let mut set = std::collections::HashSet::new();
        set.insert(9i64);

        for units in [
            TextUnits::CodePoints,
            TextUnits::Utf8Bytes,
            TextUnits::Utf16Units,
        ] {
            assert_eq!(PyLen::py_len(&list, units), 3, "{units:?}");
            assert_eq!(PyLen::py_len(&map, units), 1, "{units:?}");
            assert_eq!(PyLen::py_len(&set, units), 1, "{units:?}");
        }
    }
}

mod mapping_and_membership {
    use super::*;
    use compylr_backend_rust::runtime::{PyContains, PyIterate, PySetItem, py_key};

    fn map() -> std::collections::HashMap<String, i64> {
        let mut map = std::collections::HashMap::new();
        map.insert("present".to_string(), 1i64);
        map
    }

    #[test]
    fn a_missing_key_is_reported_and_names_itself() {
        let error = py_key(&map(), &"absent".to_string()).expect_err("not in the map");
        match error {
            RuntimeError::MissingKey(key) => assert!(key.contains("absent"), "{key}"),
            other => panic!("expected a missing key, got {other:?}"),
        }
        assert_eq!(py_key(&map(), &"present".to_string()), Ok(1));
    }

    /// Assigning creates a key that reading would have refused.
    ///
    /// The two are different operations sharing a spelling, and conflating them would either make
    /// reads create entries or make assignments fail on any key not already there.
    #[test]
    fn assigning_inserts_a_key_that_reading_refuses() {
        let mut map = map();
        assert!(py_key(&map, &"fresh".to_string()).is_err());

        map.py_set(&"fresh".to_string(), 2).unwrap();
        assert_eq!(py_key(&map, &"fresh".to_string()), Ok(2));

        map.py_set(&"fresh".to_string(), 3).unwrap();
        assert_eq!(py_key(&map, &"fresh".to_string()), Ok(3), "and overwrites");
    }

    /// A sequence has no element to create, so assigning out of range is a failure.
    #[test]
    fn assigning_past_a_sequences_end_is_reported() {
        let mut items = vec![1i64, 2];
        items.py_set(&0, 9).unwrap();
        assert_eq!(items[0], 9);
        assert_eq!(
            items.py_set(&5, 9),
            Err(RuntimeError::IndexOutOfRange),
            "a sequence does not grow to fit an assignment"
        );
    }

    #[test]
    fn membership_works_over_every_container() {
        let list = vec![1i64, 2];
        let mut set = std::collections::HashSet::new();
        set.insert(3i64);

        assert!(list.py_contains(&1));
        assert!(!list.py_contains(&9));
        assert!(set.py_contains(&3));
        assert!(!set.py_contains(&9));
        // A mapping tests its keys, not its values.
        assert!(map().py_contains(&"present".to_string()));
        assert!(!map().py_contains(&"absent".to_string()));
    }

    /// Membership in a string is a substring test, which every language compylr accepts agrees on.
    #[test]
    fn membership_in_a_string_tests_substrings() {
        let text = "cab".to_string();
        assert!(text.py_contains(&"ab".to_string()));
        assert!(text.py_contains(&"".to_string()));
        assert!(!text.py_contains(&"abc".to_string()));
    }

    #[test]
    fn iterating_a_mapping_yields_its_keys() {
        let keys: Vec<String> = map().py_iter().collect();
        assert_eq!(keys, ["present".to_string()]);
    }

    #[test]
    fn iterating_a_sequence_and_a_set_yields_their_elements() {
        let list = vec![1i64, 2];
        let mut yielded: Vec<i64> = list.py_iter().collect();
        yielded.sort_unstable();
        assert_eq!(yielded, [1, 2]);

        let mut set = std::collections::HashSet::new();
        set.insert(7i64);
        let from_set: Vec<i64> = set.py_iter().collect();
        assert_eq!(from_set, [7]);
    }
}

/// The IR's mode enums and the runtime's copies are two spellings of one decision.
///
/// They cannot be coupled: this file is embedded verbatim into generated crates and may not name
/// anything outside itself. So they are compared as text, which is the same idiom
/// `tests/crate_boundaries.rs` uses for the claims the type system cannot carry.
mod the_two_copies_agree {
    fn variants(source: &str, enum_name: &str) -> Vec<String> {
        let start = source
            .find(&format!("pub enum {enum_name} {{"))
            .unwrap_or_else(|| panic!("{enum_name} must be declared"));
        let body = &source[start..];
        let end = body.find("\n}").expect("the enum must close");

        body[..end]
            .lines()
            .skip(1)
            .filter_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .starts_with(|c: char| c.is_ascii_uppercase())
                    .then(|| trimmed.trim_end_matches(',').to_string())
            })
            .collect()
    }

    #[test]
    fn the_mode_enums_have_the_same_variants_on_both_sides() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate lives at <root>/crates/<name>")
            .to_path_buf();

        let ir = std::fs::read_to_string(root.join("crates/compylr-ir/src/ir.rs")).unwrap();
        let runtime =
            std::fs::read_to_string(root.join("crates/compylr-backend-rust/src/runtime.rs"))
                .unwrap();

        for name in ["IndexOrigin", "TextUnits"] {
            let declared = variants(&ir, name);
            assert!(!declared.is_empty(), "{name} must have variants");
            assert_eq!(
                declared,
                variants(&runtime, name),
                "{name} differs between the IR and the emitted runtime; the backend emits the \
                 IR's spelling, so a variant on one side and not the other produces generated \
                 code that does not compile"
            );
        }
    }
}
