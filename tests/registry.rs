//! The backend registry's three-way answer.
//!
//! A user asking for `typescript` and a user typoing `rsut` have made different mistakes and need
//! different messages: one is waiting on work that is planned, the other has a typo. Collapsing
//! both into "unknown backend" would tell the first user their target does not exist, which is
//! wrong and actively discouraging.

use compylr::backend::{BackendError, lookup, names};

#[test]
fn the_rust_backend_is_implemented() {
    let backend = lookup("rust").expect("rust must be implemented");
    assert_eq!(backend.name(), "rust");
}

#[test]
fn reserved_backends_resolve_as_reserved() {
    for name in ["typescript", "go", "cpp"] {
        match lookup(name) {
            Err(BackendError::NotImplemented { backend }) => assert_eq!(backend, name),
            Err(other) => panic!("{name} should be reserved, got {other:?}"),
            Ok(_) => panic!("{name} is not implemented yet and must not resolve to a backend"),
        }
    }
}

#[test]
fn an_unrecognized_backend_lists_the_available_names() {
    match lookup("brainfuck") {
        Err(BackendError::Unknown { backend, available }) => {
            assert_eq!(backend, "brainfuck");
            assert!(
                available.contains(&"rust".to_string()),
                "the error must name the backends a user can actually pick, got {available:?}"
            );
        }
        other => panic!("expected an unknown-backend error, got {other:?}"),
    }
}

#[test]
fn reserved_and_unknown_are_distinguishable_without_matching_message_text() {
    let reserved = lookup("typescript").unwrap_err();
    let unknown = lookup("nonesuch").unwrap_err();

    assert!(reserved.is_not_implemented());
    assert!(!reserved.is_unknown());
    assert!(unknown.is_unknown());
    assert!(!unknown.is_not_implemented());
}

#[test]
fn the_reserved_error_says_it_is_not_implemented_yet() {
    let message = lookup("typescript").unwrap_err().to_string();
    assert!(
        message.contains("not implemented yet"),
        "a reserved backend must read as planned-but-absent, got: {message}"
    );
    assert!(message.contains("typescript"));
}

#[test]
fn the_unknown_error_does_not_claim_the_backend_is_planned() {
    let message = lookup("nonesuch").unwrap_err().to_string();
    assert!(
        !message.contains("not implemented yet"),
        "a typo must not be reported as a planned backend, got: {message}"
    );
    assert!(
        message.contains("rust"),
        "it should suggest what is available"
    );
}

#[test]
fn names_covers_every_registry_entry() {
    let all = names();
    for expected in ["rust", "typescript", "go", "cpp"] {
        assert!(all.contains(&expected), "registry is missing {expected}");
    }
}

#[test]
fn lookup_is_case_sensitive_and_does_not_guess() {
    // Accepting `Rust` would mean deciding what else to normalise, and a backend name arrives
    // from a config value the user typed deliberately.
    assert!(lookup("Rust").unwrap_err().is_unknown());
}
