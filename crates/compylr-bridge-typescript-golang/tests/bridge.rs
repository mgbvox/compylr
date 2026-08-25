use compylr_bridge_typescript_golang::TypeScriptGoBridge;
use compylr_core::HostBridge;
use compylr_core::bridge::BuildKey;
use compylr_ir::{BinOp, Checked, Expr, Function, Param, Stmt, Ty, Unit};

#[test]
fn bridge_emits_cgo_wrapper_and_ts_loader() {
    let bridge = TypeScriptGoBridge;
    let mut unit = Unit::new();
    let func = Function {
        name: "multiply".to_string(),
        params: vec![
            Param {
                name: "x".to_string(),
                ty: Ty::Int,
            },
            Param {
                name: "y".to_string(),
                ty: Ty::Int,
            },
        ],
        ret: Ty::Int,
        body: vec![Stmt::Return(Expr::Binary {
            op: BinOp::Mul {
                checked: Checked::Unchecked,
            },
            left: Box::new(Expr::Name("x".to_string())),
            right: Box::new(Expr::Name("y".to_string())),
        })],
        doc: None,
        span: Default::default(),
    };
    unit.add_function(func).unwrap();

    let build_key = BuildKey {
        fingerprint: 12345,
        target: "go".to_string(),
        passes: "default".to_string(),
    };

    let artifact = bridge.emit(&unit, &build_key).expect("bridge emit failed");
    assert_eq!(artifact.files.len(), 6);
    assert!(artifact.files.contains_key("go.mod"));
    assert!(artifact.files.contains_key("compat.go"));
    assert!(artifact.files.contains_key("generated.go"));
    assert!(artifact.files.contains_key("bindings.go"));
    assert!(artifact.files.contains_key("index.d.ts"));
    assert!(artifact.files.contains_key("index.js"));

    let go_bridge = artifact
        .files
        .get("bindings.go")
        .expect("bindings.go missing");
    assert!(go_bridge.contains("import \"C\""));
    assert!(go_bridge.contains("//export Call_multiply"));
    assert!(go_bridge.contains("func Call_multiply(x C.longlong, y C.longlong) C.longlong"));

    let js_loader = artifact.files.get("index.js").expect("index.js missing");
    assert!(js_loader.contains("koffi"));
    assert!(js_loader.contains("function multiply(x, y)"));
}
