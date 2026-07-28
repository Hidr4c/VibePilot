#![windows_subsystem = "windows"]
#![allow(unused_imports, unreachable_patterns, clippy::too_many_arguments, clippy::type_complexity, clippy::new_without_default)]

//! VibePilot Grid Master - Multi-agent GUI orchestrator scheduler.

pub mod master;

// Re-use core models and utilities from target crate
pub mod config;
pub mod defaults;
pub mod memory;
pub mod event_bus;
pub mod ocr;
pub mod screen_capture;
pub mod peripheral_controller;
pub mod services;
pub mod llm_client;
pub mod vision;
pub mod secure_store;
pub mod append_store;
pub mod reflection;
pub mod vision_pipeline;
pub mod decision_router;
pub mod error;
pub mod version;
pub mod prompt_templates;
pub mod content;
pub mod app;
pub mod orchestrator;
pub mod app_services;
pub mod ui;
pub mod commands;
pub mod peripheral;
pub mod macro_recorder;

use master::MasterApp;

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
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([600.0, 450.0])
            .with_resizable(true),
        ..Default::default()
    };

    let rt = tokio::runtime::Runtime::new().expect("Failed to build Tokio runtime for Master");
    let is_test = cfg!(test)
        || std::env::var("VIBEPILOT_TEST").is_ok()
        || std::thread::current().name().unwrap_or("main").contains("test")
        || std::env::args().skip(1).any(|arg| arg.contains("test"));

    let base_dir = match crate::config::load_bootstrap_config().storage_dir {
        Some(dir_str) if !dir_str.is_empty() => std::path::PathBuf::from(&dir_str),
        _ => {
            if is_test {
                let thread_name = std::thread::current().name().unwrap_or("main").to_string();
                let safe_name = thread_name.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                    .map(|c| if c == ':' { '_' } else { c })
                    .collect::<String>();
                std::env::temp_dir().join(format!("vibepilot_test_{}", safe_name))
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        }
    };
    let config_repo = crate::config::ConfigRepositoryFactory::create(base_dir);

    eframe::run_native(
        "VibePilot Grid Master",
        native_options,
        Box::new(move |cc| Ok(Box::new(MasterApp::new(&cc.egui_ctx, config_repo, rt)))),
    )
}

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
