//! Node-API host extension module for compylr.

use napi_derive::napi;

#[napi]
pub fn version() -> String {
    "0.1.0".to_string()
}
