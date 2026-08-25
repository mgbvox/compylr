//! How TypeScript types and operators are spelled in compiler diagnostics.

use compylr_ir::{BinOp, Ty};

pub trait TypeScriptSpelling {
    fn typescript_name(&self) -> String;
}

impl TypeScriptSpelling for Ty {
    fn typescript_name(&self) -> String {
        match self {
            Self::Int | Self::Float => "number".to_string(),
            Self::Bool => "boolean".to_string(),
            Self::Str => "string".to_string(),
            Self::Unit => "void".to_string(),
            Self::List(elem) => format!("Array<{}>", elem.typescript_name()),
            Self::Dict(k, v) => format!("Map<{}, {}>", k.typescript_name(), v.typescript_name()),
            Self::Set(elem) => format!("Set<{}>", elem.typescript_name()),
            Self::Tuple(elems) => {
                let inner = elems
                    .iter()
                    .map(|t| t.typescript_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            Self::Instance(name) => name.clone(),
        }
    }
}

impl TypeScriptSpelling for BinOp {
    fn typescript_name(&self) -> String {
        match self {
            Self::Add { .. } => "+".to_string(),
            Self::Sub { .. } => "-".to_string(),
            Self::Mul { .. } => "*".to_string(),
            Self::Div { .. } => "/".to_string(),
            Self::Rem { .. } => "%".to_string(),
            Self::Eq => "===".to_string(),
            Self::NotEq => "!==".to_string(),
            Self::Lt => "<".to_string(),
            Self::LtE => "<=".to_string(),
            Self::Gt => ">".to_string(),
            Self::GtE => ">=".to_string(),
        }
    }
}
