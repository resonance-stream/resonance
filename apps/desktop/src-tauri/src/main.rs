//! Resonance Desktop Application Entry Point
//!
//! This is the main entry point for the Resonance desktop application.
//! It initializes the Tauri runtime and launches the application window.

// Prevents additional console window on Windows in release builds
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    resonance_desktop::run();
}
