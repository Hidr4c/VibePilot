//! Storage persistence for macro sequences, sessions (runs), and hotkeys.

use super::{HotkeyConfig, MacroSequence, MacroSessionManager};
use std::fs;
use std::path::Path;

pub struct MacroStorage;

impl MacroStorage {
    /// Saves a macro sequence to a JSON file.
    pub fn save(sequence: &MacroSequence, path: impl AsRef<Path>) -> Result<(), String> {
        let content = serde_json::to_string_pretty(sequence)
            .map_err(|e| format!("Failed to serialize macro sequence: {e}"))?;
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content)
            .map_err(|e| format!("Failed to write macro sequence file: {e}"))
    }

    /// Loads a macro sequence from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<MacroSequence, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read macro sequence file: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse macro sequence JSON: {e}"))
    }

    /// Saves all session runs to a JSON file.
    pub fn save_sessions(manager: &MacroSessionManager, path: impl AsRef<Path>) -> Result<(), String> {
        let content = serde_json::to_string_pretty(manager)
            .map_err(|e| format!("Failed to serialize macro session manager: {e}"))?;
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content)
            .map_err(|e| format!("Failed to write macro sessions file: {e}"))
    }

    /// Loads all session runs from a JSON file.
    pub fn load_sessions(path: impl AsRef<Path>) -> Result<MacroSessionManager, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read macro sessions file: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse macro sessions JSON: {e}"))
    }

    /// Saves global hotkey configuration to a JSON file.
    pub fn save_hotkeys(config: &HotkeyConfig, path: impl AsRef<Path>) -> Result<(), String> {
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize hotkey config: {e}"))?;
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content)
            .map_err(|e| format!("Failed to write hotkey config file: {e}"))
    }

    /// Loads global hotkey configuration from a JSON file.
    pub fn load_hotkeys(path: impl AsRef<Path>) -> Result<HotkeyConfig, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read hotkey config file: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse hotkey config JSON: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macro_recorder::ActionKind;

    #[test]
    fn test_macro_storage_save_load() {
        let temp = std::env::temp_dir().join("vibepilot_macro_test.json");
        let mut seq = MacroSequence::new("Saved Sequence");
        seq.add_action(50, ActionKind::Wait { ms: 100 });

        assert!(MacroStorage::save(&seq, &temp).is_ok());
        let loaded = MacroStorage::load(&temp);
        assert!(loaded.is_ok());
        let loaded_seq = loaded.unwrap();
        assert_eq!(loaded_seq.name, "Saved Sequence");
        assert_eq!(loaded_seq.actions.len(), 1);

        let _ = fs::remove_file(temp);
    }

    #[test]
    fn test_macro_session_storage_save_load() {
        let temp = std::env::temp_dir().join("vibepilot_sessions_test.json");
        let mgr = MacroSessionManager::new();

        assert!(MacroStorage::save_sessions(&mgr, &temp).is_ok());
        let loaded = MacroStorage::load_sessions(&temp);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().runs.len(), 1);

        let _ = fs::remove_file(temp);
    }

    #[test]
    fn test_macro_hotkey_storage_save_load() {
        let temp = std::env::temp_dir().join("vibepilot_hotkeys_test.json");
        let mut cfg = HotkeyConfig::default();
        cfg.start_recording_keys = vec!["Alt".to_string(), "S".to_string()];

        assert!(MacroStorage::save_hotkeys(&cfg, &temp).is_ok());
        let loaded = MacroStorage::load_hotkeys(&temp);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().start_recording_keys, vec!["Alt", "S"]);

        let _ = fs::remove_file(temp);
    }
}
