//! Tauri build script
//!
//! This script is run by Cargo before building the main crate.
//! It is required for Tauri to properly compile resources and generate bindings.

fn main() {
    tauri_build::build();
}
