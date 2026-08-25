use compylr_registry::{backends, bridges, frontends};

#[test]
fn resolves_typescript_frontend() {
    let fe = frontends::lookup("typescript").expect("lookup failed");
    assert_eq!(fe.name(), "typescript");
    assert!(frontends::implemented_names().contains(&"typescript".to_string()));
}

#[test]
fn resolves_go_backend() {
    let be = backends::lookup("go").expect("lookup failed");
    assert_eq!(be.name(), "go");
    assert!(backends::implemented_names().contains(&"go".to_string()));
}

#[test]
fn resolves_typescript_go_bridge() {
    let bridge = bridges::lookup("typescript", "go").expect("lookup failed");
    assert_eq!(bridge.source(), "typescript");
    assert_eq!(bridge.target(), "go");
    assert!(bridges::pairs().contains(&("typescript".to_string(), "go".to_string())));
}
