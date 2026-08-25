use std::fmt::Write as _;

use compylr_backend_golang::GoBackend;
use compylr_core::backend::{Backend, BackendError};
use compylr_core::bridge::{BuildKey, HostArtifact, HostBridge};
use compylr_ir::{Function, Ty, Unit};

/// Host bridge connecting TypeScript source to Go backend.
#[derive(Debug)]
pub struct TypeScriptGoBridge;

impl HostBridge for TypeScriptGoBridge {
    fn source(&self) -> &'static str {
        "typescript"
    }

    fn target(&self) -> &'static str {
        "go"
    }

    fn emit(&self, unit: &Unit, key: &BuildKey) -> Result<HostArtifact, BackendError> {
        let backend = GoBackend;
        let mut files = backend.emit(unit)?;

        let loaded_as = format!(
            "compylr_generated_{:016x}_{}",
            key.fingerprint,
            key.variant_tag()
        );

        // Emit CGo bindings.go
        let bindings_go = emit_cgo_bindings(unit);
        files.insert("bindings.go".to_string(), bindings_go);

        // Emit TypeScript declarations
        let d_ts = emit_dts(unit);
        files.insert("index.d.ts".to_string(), d_ts);

        // Emit JS FFI Loader
        let js_loader = emit_js_loader(unit, &loaded_as);
        files.insert("index.js".to_string(), js_loader);

        let manifest = files.get("go.mod").cloned().unwrap_or_default();

        Ok(HostArtifact {
            files,
            manifest,
            loaded_as,
        })
    }
}

fn emit_cgo_bindings(unit: &Unit) -> String {
    let mut out = String::new();
    writeln!(out, "package main\n").unwrap();
    writeln!(out, "/*").unwrap();
    writeln!(out, "#include <stdlib.h>").unwrap();
    writeln!(out, "*/").unwrap();
    writeln!(out, "import \"C\"\n").unwrap();

    for func in unit.functions() {
        emit_cgo_function(func, &mut out);
    }

    writeln!(out, "func main() {{}}").unwrap();
    out
}

fn is_scalar(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float | Ty::Bool | Ty::Unit)
}

fn emit_cgo_function(func: &Function, out: &mut String) {
    if !func.params.iter().all(|p| is_scalar(&p.ty)) || !is_scalar(&func.ret) {
        return;
    }
    let fn_name = &func.name;
    let cgo_name = format!("Call_{}", fn_name);

    let mut c_params = Vec::new();
    let mut call_args = Vec::new();
    for p in &func.params {
        match &p.ty {
            Ty::Int => {
                c_params.push(format!("{} C.longlong", p.name));
                call_args.push(format!("int64({})", p.name));
            }
            Ty::Float => {
                c_params.push(format!("{} C.double", p.name));
                call_args.push(format!("float64({})", p.name));
            }
            Ty::Bool => {
                c_params.push(format!("{} C.longlong", p.name));
                call_args.push(format!("{} != 0", p.name));
            }
            _ => {}
        }
    }
    let c_params_str = c_params.join(", ");

    let ret_type = match &func.ret {
        Ty::Float => "C.double",
        _ => "C.longlong",
    };

    writeln!(out, "//export {}", cgo_name).unwrap();
    writeln!(out, "func {}({}) {} {{", cgo_name, c_params_str, ret_type).unwrap();
    if func.ret == Ty::Unit {
        writeln!(out, "\t{}({})", fn_name, call_args.join(", ")).unwrap();
        writeln!(out, "\treturn 0").unwrap();
    } else if func.ret == Ty::Bool {
        writeln!(out, "\tres := {}({})", fn_name, call_args.join(", ")).unwrap();
        writeln!(out, "\tif res {{ return 1 }} else {{ return 0 }}").unwrap();
    } else if func.ret == Ty::Float {
        writeln!(out, "\tres := {}({})", fn_name, call_args.join(", ")).unwrap();
        writeln!(out, "\treturn C.double(res)").unwrap();
    } else {
        writeln!(out, "\tres := {}({})", fn_name, call_args.join(", ")).unwrap();
        writeln!(out, "\treturn C.longlong(res)").unwrap();
    }
    writeln!(out, "}}\n").unwrap();
}

fn emit_dts(unit: &Unit) -> String {
    let mut out = String::new();
    for func in unit.functions() {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: number", p.name))
            .collect();
        let ret = if func.ret == Ty::Unit {
            "void"
        } else {
            "number"
        };
        writeln!(
            out,
            "export function {}({}): {};",
            func.name,
            params.join(", "),
            ret
        )
        .unwrap();
    }
    out
}

fn emit_js_loader(unit: &Unit, module_name: &str) -> String {
    let mut out = String::new();
    writeln!(out, "// FFI Loader for {}", module_name).unwrap();
    writeln!(out, "const path = require('path');").unwrap();
    writeln!(out, "const koffi = require('koffi');\n").unwrap();
    writeln!(
        out,
        "const libPath = path.join(__dirname, '{}.so');",
        module_name
    )
    .unwrap();
    writeln!(out, "const lib = koffi.load(libPath);\n").unwrap();

    for func in unit.functions() {
        let cgo_name = format!("Call_{}", func.name);
        let params_sig: Vec<&str> = func.params.iter().map(|_| "'int64'").collect();
        let ret_sig = "'int64'";
        writeln!(
            out,
            "const native_{} = lib.func('{}', {}, [{}]);",
            func.name,
            cgo_name,
            ret_sig,
            params_sig.join(", ")
        )
        .unwrap();

        let params: Vec<&str> = func.params.iter().map(|p| p.name.as_str()).collect();
        writeln!(
            out,
            "function {}({}) {{ return native_{}({}); }}",
            func.name,
            params.join(", "),
            func.name,
            params.join(", ")
        )
        .unwrap();
        writeln!(out, "exports.{} = {};\n", func.name, func.name).unwrap();
    }
    out
}
