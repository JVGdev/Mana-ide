//! The engine, as the spell tester sees it (PLAN step R6).

use wasm_bindgen::prelude::*;

/// The engine's version: for the tester to check it loaded.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
