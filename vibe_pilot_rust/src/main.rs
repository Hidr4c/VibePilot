#![windows_subsystem = "windows"]
#![allow(unused_imports, unreachable_patterns, clippy::too_many_arguments, clippy::type_complexity, clippy::new_without_default, clippy::needless_borrows_for_generic_args, clippy::collapsible_if, clippy::if_same_then_else, clippy::manual_swap, clippy::field_reassign_with_default, clippy::unnecessary_unwrap, clippy::get_first, clippy::needless_match, clippy::collapsible_str_replace)]

//! VibePilot - Visual AI automation orchestrator (Rust port)
//!
//! Replaces the Python/Tkinter implementation with a lightweight,
//! native Rust application using egui for the GUI.

pub mod app;
pub mod app_services;
pub mod commands;
pub mod config;
pub mod defaults;
pub mod content;
pub mod event_bus;
pub mod llm_client;
pub mod orchestrator;
pub mod peripheral;
pub mod peripheral_controller;
pub mod screen_capture;
pub mod memory;
pub mod reflection;
pub mod vision_pipeline;
pub mod decision_router;
pub mod secure_store;
pub mod append_store;
pub mod ui;
pub mod version;
pub mod error;
pub mod services;
pub mod prompt_templates;
pub mod ocr;
pub mod vision;


use app::VibePilotApp;

#[cfg_attr(tarpaulin, skip)]
fn main() -> eframe::Result {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE};
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
    }

    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([400.0, 300.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "VibePilot",
        native_options,
        Box::new(|cc| Ok(Box::new(VibePilotApp::new(&cc.egui_ctx)))),
    )
}

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
