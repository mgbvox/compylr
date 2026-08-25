//! Type mappings and identifier sanitization for Go.

use compylr_ir::Ty;

const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
    "error",
    "nil",
    "iota",
];

/// Sanitize Go identifier.
pub fn go_ident(name: &str) -> String {
    if GO_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Convert compylr Ty to Go type string.
pub fn go_ty(ty: &Ty) -> String {
    match ty {
        Ty::Int => "int64".to_string(),
        Ty::Float => "float64".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::Str => "string".to_string(),
        Ty::Unit => "struct{}".to_string(),
        Ty::List(elem) => format!("[]{}", go_ty(elem)),
        Ty::Dict(k, v) => format!("map[{}]{}", go_ty(k), go_ty(v)),
        Ty::Set(elem) => format!("map[{}]struct{{}}", go_ty(elem)),
        Ty::Instance(cls) => format!("*{}", go_ident(cls)),
        Ty::Tuple(elems) => {
            let fields: Vec<String> = elems
                .iter()
                .enumerate()
                .map(|(i, t)| format!("F{} {}", i, go_ty(t)))
                .collect();
            format!("struct {{ {} }}", fields.join("; "))
        }
    }
}
