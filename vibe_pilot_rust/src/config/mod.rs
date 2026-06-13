pub mod models;
pub mod helpers;
pub mod repository;
pub mod factory;
pub mod shared_state;

#[cfg(test)]
pub mod tests;

use std::path::PathBuf;
use crate::memory::TaskGraph;

pub use models::{
    EngineProfile, EnginePresets, SavedConfig, PersistentState,
    SessionSnapshot, BootstrapConfig,
    ENGINE_LM_STUDIO, ENGINE_OLLAMA, ENGINE_CUSTOM,
    LANG_FR, LANG_EN,
    ACTION_THINK, ACTION_WAIT, ACTION_COOL, ACTION_SCROLL, ACTION_CLICK_AND_TYPE, ACTION_SUCCESS, ACTION_FAIL, ACTION_SLEEP, ACTION_STATIC,
    COLOR_THINK, COLOR_WAIT, COLOR_COOL, COLOR_SCROLL, COLOR_CLICK, COLOR_SUCCESS, COLOR_FAIL, COLOR_SLEEP, COLOR_STATIC,
    COLOR_GRAY, COLOR_DARK_GREEN, COLOR_BACKGROUND_DARK, COLOR_TERMINAL_BG, COLOR_TERMINAL_FG,
    BG_BUTTON_TEAL, BG_BUTTON_RED, BG_BUTTON_GREEN, BG_BUTTON_ORANGE, BG_BUTTON_PURPLE,
    BG_BUTTON_SLATE, BG_BUTTON_STEEL, BG_BUTTON_DARK_SLATE, BG_BUTTON_BLUE, BG_BUTTON_GRAY, BG_BUTTON_ORANGE_RED,
    FALLBACK_BBOX, ALL_SCREENS_KEY, APP_NAME,
    DEFAULT_CONTEXT, DEFAULT_DIRECTIVES, DEFAULT_OBJECTIF, DEFAULT_TASK, DEFAULT_DEMANDE_GENERIQUE,
};

pub use helpers::{
    contains_dangerous_command, is_dangerous_shortcut, export_decrypted_journal,
    load_bootstrap_config, save_bootstrap_config, get_bootstrap_config_path,
    dpapi, ActionLogger,
};

pub use repository::ConfigRepository;
pub use factory::ConfigRepositoryFactory;
pub use shared_state::{SharedStateManager, FileSharedStateManager, SharedStateManagerFactory};

// ============================================================
// Focused traits (Interface Segregation Principle)
// ============================================================

/// Configuration save/load operations (core config + engines).
pub trait ConfigManagement: Send + Sync {
    fn load_config(&self) -> SavedConfig;
    fn save_config(&self, config: &SavedConfig);
    fn save_now(&self) -> Result<(), String>;

    // Path helpers needed by ConfigManagement
    fn get_save_path(&self) -> PathBuf;
    fn get_key_path(&self) -> PathBuf;
}

/// Profile management operations.
pub trait ProfileManagement: Send + Sync {
    fn load_profile(&self, name: &str) -> Option<SavedConfig>;
    fn save_profile(&self, name: &str, config: &SavedConfig) -> bool;
    fn delete_profile(&self, name: &str) -> bool;
    fn list_profiles(&self) -> Vec<String>;

    // Path helpers needed by ProfileManagement
    fn get_profiles_dir(&self) -> PathBuf;

    // Export / import
    fn export_profiles(&self, path: &std::path::Path) -> Result<(), String>;
    fn import_profiles(&self, path: &std::path::Path) -> Result<usize, String>;
    fn export_single_profile(&self, name: &str, path: &std::path::Path) -> Result<(), String>;
    fn import_single_profile(&self, path: &std::path::Path) -> Result<String, String>;
}

/// Engine preset management operations.
pub trait EngineManagement: Send + Sync {
    fn load_engines(&self) -> EnginePresets;
    fn save_engines(&self, presets: &EnginePresets);

    // Path helpers needed by EngineManagement
    fn get_engines_path(&self) -> PathBuf;

    // Export / import
    fn export_engines(&self, path: &std::path::Path) -> Result<(), String>;
    fn import_engines(&self, path: &std::path::Path) -> Result<(), String>;
    fn export_single_engine(&self, name: &str, path: &std::path::Path) -> Result<(), String>;
    fn import_single_engine(&self, path: &std::path::Path) -> Result<String, String>;
}

/// State management (task graph, full config export/import).
pub trait StateManagement: Send + Sync {
    fn load_task_graph(&self) -> Option<TaskGraph>;
    fn save_task_graph(&self, graph: Option<TaskGraph>);
    fn export_config(&self, path: &std::path::Path) -> Result<(), String>;
    fn import_config(&self, path: &std::path::Path) -> Result<(), String>;
}

/// Base trait for path queries.
pub trait ConfigPaths: Send + Sync {
    fn get_base_dir(&self) -> PathBuf;
    fn get_store_path(&self) -> PathBuf;
}

/// Composite trait extending all focused traits + path queries.
pub trait ConfigurationRepository:
    ConfigManagement
    + ProfileManagement
    + EngineManagement
    + StateManagement
    + ConfigPaths
{
}

// Blanket impl: anything implementing all sub-traits implements ConfigurationRepository
impl<T> ConfigurationRepository for T where
    T: ConfigManagement
        + ProfileManagement
        + EngineManagement
        + StateManagement
        + ConfigPaths,
{
}
