#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! VibePilot - Visual AI automation orchestrator (Rust port)
//!
//! Replaces the Python/Tkinter implementation with a lightweight,
//! native Rust application using egui for the GUI.

pub mod app;
pub mod config;
pub mod defaults;
pub mod content;
pub mod event_bus;
pub mod llm_client;
pub mod orchestrator;
pub mod peripheral_controller;
pub mod screen_capture;
pub mod ui;
pub mod version;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod common {
    use std::path::PathBuf;
    use std::fs;

    pub fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn cleanup(name: &str) {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
    }
}

use app::VibePilotApp;

fn main() -> eframe::Result {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "VibePilot",
        native_options,
        Box::new(|cc| Ok(Box::new(VibePilotApp::new(&cc.egui_ctx)))),
    )
}
