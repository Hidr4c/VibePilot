//! Configuration persistence for VibePilot.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod dpapi {
    use std::ffi::c_void;
    use std::ptr;

    #[repr(C)]
    struct DATA_BLOB {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            pDataIn: *const DATA_BLOB,
            pszDataDescr: *const u16,
            pOptionalEntropy: *const DATA_BLOB,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DATA_BLOB,
        ) -> i32;

        fn CryptUnprotectData(
            pDataIn: *const DATA_BLOB,
            ppszDataDescr: *mut *mut u16,
            pOptionalEntropy: *const DATA_BLOB,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DATA_BLOB,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(hMem: *mut c_void) -> *mut c_void;
    }

    pub fn encrypt(data: &[u8]) -> Option<Vec<u8>> {
        let data_in = DATA_BLOB {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let mut data_out = DATA_BLOB {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };

        unsafe {
            let success = CryptProtectData(
                &data_in,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                0x1, // CRYPTPROTECT_UI_FORBIDDEN
                &mut data_out,
            );

            if success != 0 && !data_out.pb_data.is_null() {
                let result = std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize).to_vec();
                LocalFree(data_out.pb_data as *mut c_void);
                Some(result)
            } else {
                None
            }
        }
    }

    pub fn decrypt(data: &[u8]) -> Option<Vec<u8>> {
        let data_in = DATA_BLOB {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let mut data_out = DATA_BLOB {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };

        unsafe {
            let success = CryptUnprotectData(
                &data_in,
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                0x1, // CRYPTPROTECT_UI_FORBIDDEN
                &mut data_out,
            );

            if success != 0 && !data_out.pb_data.is_null() {
                let result = std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize).to_vec();
                LocalFree(data_out.pb_data as *mut c_void);
                Some(result)
            } else {
                None
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod dpapi {
    pub fn encrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
    pub fn decrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

fn load_secure_data<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<(T, bool)> {
    if let Ok(bytes) = fs::read(path) {
        if let Some(decrypted) = dpapi::decrypt(&bytes) {
            if let Ok(decrypted_str) = std::str::from_utf8(&decrypted) {
                if let Ok(data) = toml::from_str::<T>(decrypted_str) {
                    return Some((data, true));
                }
            }
        }
        if let Ok(bytes_str) = std::str::from_utf8(&bytes) {
            if let Ok(data) = toml::from_str::<T>(bytes_str) {
                return Some((data, false));
            }
        }
    }
    None
}

fn save_secure_data<T: serde::Serialize>(path: &std::path::Path, data: &T) -> bool {
    if let Ok(toml_str) = toml::to_string(data) {
        if let Some(encrypted) = dpapi::encrypt(toml_str.as_bytes()) {
            return fs::write(path, encrypted).is_ok();
        } else {
            return fs::write(path, toml_str.as_bytes()).is_ok();
        }
    }
    false
}

/// Engine profile with dynamic key support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineProfile {
    pub url: String,
    pub modeles: Vec<String>,
}

/// Default engine presets (LM Studio, Ollama, Custom) + dynamic custom engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginePresets {
    #[serde(default = "default_lm_studio")]
    pub lm_studio: EngineProfile,
    #[serde(default = "default_ollama")]
    pub ollama: EngineProfile,
    #[serde(default = "default_custom")]
    pub custom: EngineProfile,
    #[serde(default = "default_custom_engines")]
    pub custom_engines: HashMap<String, EngineProfile>,
}

impl Default for EnginePresets {
    fn default() -> Self {
        Self {
            lm_studio: default_lm_studio(),
            ollama: default_ollama(),
            custom: default_custom(),
            custom_engines: HashMap::new(),
        }
    }
}

fn default_lm_studio() -> EngineProfile {
    EngineProfile {
        url: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
        modeles: vec![
            "qwen/qwen3.6-35b-a3b".to_string(),
            "qwen2.5-vl-7b-instruct".to_string(),
            "meta-llama-3-8b-instruct".to_string(),
        ],
    }
}

fn default_ollama() -> EngineProfile {
    EngineProfile {
        url: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
        modeles: vec![
            "qwen2.5-coder:7b".to_string(),
            "qwen2.5-coder:32b".to_string(),
            "llava:latest".to_string(),
            "llama3:latest".to_string(),
        ],
    }
}

fn default_custom() -> EngineProfile {
    EngineProfile {
        url: "http://localhost:8000/v1/chat/completions".to_string(),
        modeles: vec!["custom-model-name".to_string()],
    }
}

fn default_custom_engines() -> HashMap<String, EngineProfile> {
    HashMap::new()
}

impl EnginePresets {
    pub fn all_keys(&self) -> Vec<String> {
        let mut keys = vec!["LM Studio".to_string(), "Ollama".to_string(), "Perso / Autre".to_string()];
        keys.extend(self.custom_engines.keys().cloned());
        keys
    }

    pub fn get(&self, name: &str) -> Option<EngineProfile> {
        if name == "LM Studio" {
            return Some(self.lm_studio.clone());
        }
        if name == "Ollama" {
            return Some(self.ollama.clone());
        }
        if name == "Perso / Autre" {
            return Some(self.custom.clone());
        }
        self.custom_engines.get(name).cloned()
    }

    pub fn insert(&mut self, name: String, profile: EngineProfile) {
        self.custom_engines.insert(name, profile);
    }

    pub fn remove(&mut self, name: &str) {
        self.custom_engines.remove(name);
    }
}

// === Constants ===
pub const ENGINE_LM_STUDIO: &str = "LM Studio";
pub const ENGINE_OLLAMA: &str = "Ollama";
pub const ENGINE_CUSTOM: &str = "Perso / Autre";
pub const LANG_FR: &str = "Français";
pub const LANG_EN: &str = "English";
pub const ACTION_THINK: &str = "THINK";
pub const ACTION_WAIT: &str = "WAIT";
pub const ACTION_COOL: &str = "COOL";
pub const ACTION_SCROLL: &str = "SCROLL";
pub const ACTION_CLICK_AND_TYPE: &str = "CLICK_AND_TYPE";
pub const ACTION_SUCCESS: &str = "SUCCESS";
pub const ACTION_FAIL: &str = "FAIL";
pub const ACTION_SLEEP: &str = "SLEEP";
pub const ACTION_STATIC: &str = "STATIC";
pub const COLOR_THINK: &str = "#FF8C00";
pub const COLOR_WAIT: &str = "#1E90FF";
pub const COLOR_COOL: &str = "#00CED1";
pub const COLOR_SCROLL: &str = "#BA55D3";
pub const COLOR_CLICK: &str = "#32CD32";
pub const COLOR_SUCCESS: &str = "#FFD700";
pub const COLOR_FAIL: &str = "#FF4500";
pub const COLOR_SLEEP: &str = "#404040";
pub const COLOR_STATIC: &str = "#778899";
pub const COLOR_GRAY: &str = "#A9A9A9";
pub const COLOR_DARK_GREEN: &str = "#008000";
pub const COLOR_BACKGROUND_DARK: &str = "#202020";
pub const COLOR_TERMINAL_BG: &str = "#1E1E1E";
pub const COLOR_TERMINAL_FG: &str = "#00FF00";
pub const BG_BUTTON_TEAL: &str = "#20B2AA";
pub const BG_BUTTON_RED: &str = "#CD5C5C";
pub const BG_BUTTON_GREEN: &str = "#008000";
pub const BG_BUTTON_ORANGE: &str = "#FF8C00";
pub const BG_BUTTON_PURPLE: &str = "#7B1FA2";
pub const BG_BUTTON_SLATE: &str = "#708090";
pub const BG_BUTTON_STEEL: &str = "#4682B4";
pub const BG_BUTTON_DARK_SLATE: &str = "#2F4F4F";
pub const BG_BUTTON_BLUE: &str = "#1E90FF";
pub const BG_BUTTON_GRAY: &str = "#A9A9A9";
pub const BG_BUTTON_ORANGE_RED: &str = "#FF4500";
pub const FALLBACK_BBOX: &str = "(0, 0, 1920, 1080)";
pub const ALL_SCREENS_KEY: &str = "ALL SCREENS (Virtual Desktop)";
pub const APP_NAME: &str = "VibePilot";
pub const DEFAULT_CONTEXT: &str = "Sprint 19 has been launched in Kilo Code extension inside VS Code. The sprint is partially completed. The AI must retrieve the remaining tasks from the Kilo sprint dashboard and continue executing them sequentially.";
pub const DEFAULT_DIRECTIVES: &str = "- If you detect 'Considering next step...', 'Running...', or active test logs: the application is BUSY. Return 'WAIT'.\n- If the chat looks idle but the bottom is cut off, execute a 'SCROLL' action downwards (scroll_value: -6) to inspect the fold.\n- If and only if the application has completely stopped and provided the next prompt/input description (e.g., 'Prompt 18: ...'), extract it exactly and return 'CLICK_AND_TYPE'.";
pub const DEFAULT_OBJECTIF: &str = "test coverage up to 80%.";
pub const DEFAULT_TASK: &str = "You are the PC operator. Your target goal is to use Kilo extension in VS Code to bring test coverage up to 80%.";
pub const DEFAULT_DEMANDE_GENERIQUE: &str = "Avoir une couverture de test a 80% avec l'extension Kilo dans VS Code.";

// === SavedConfig ===
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SavedConfig {
    pub contexte: String,
    pub objectif: String,
    pub task: String,
    pub directives: String,
    pub demande_generique: String,
    pub fenetres_surveillees: Vec<String>,
    pub moteur: String,
    pub url_api: String,
    pub nom_modele: String,
    pub activer_son: bool,
    pub activer_tooltips: bool,
    pub langue: String,
    pub auto_validate: bool,
    pub auto_validate_dangerous: bool,
    pub prompt_reprise: Option<String>,
    pub theme_sombre: bool,
    pub zoom_facteur: Option<f32>,
}

impl Default for SavedConfig {
    fn default() -> Self {
        SavedConfig {
            contexte: DEFAULT_CONTEXT.to_string(),
            objectif: DEFAULT_OBJECTIF.to_string(),
            task: DEFAULT_TASK.to_string(),
            directives: DEFAULT_DIRECTIVES.to_string(),
            demande_generique: DEFAULT_DEMANDE_GENERIQUE.to_string(),
            fenetres_surveillees: vec![ALL_SCREENS_KEY.to_string()],
            moteur: ENGINE_LM_STUDIO.to_string(),
            url_api: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
            nom_modele: "qwen/qwen3.6-35b-a3b".to_string(),
            activer_son: true,
            activer_tooltips: true,
            langue: "English".to_string(),
            auto_validate: true,
            auto_validate_dangerous: false,
            prompt_reprise: None,
            theme_sombre: true,
            zoom_facteur: None,
        }
    }
}

/// Checks if a string contains any dangerous terminal command keywords.
pub fn contains_dangerous_command(text: &str) -> bool {
    let dangerous_keywords = [
        "format", "shutdown", "sudo", "rm -rf", "rm ", "drop table",
        "delete from", "chmod", "wget http", "curl http",
        "pip install", "npm install", "taskkill", "reg delete",
        "kill -9", "desactiver antivirus", "turn off firewall",
        "rmdir", "apt remove", "apt purge", "bcdedit", "dd if=",
    ];
    let lower = text.to_lowercase();
    dangerous_keywords.iter().any(|kw| lower.contains(kw))
}

// === Bootstrap Config ===
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapConfig {
    pub storage_dir: Option<String>,
}

pub fn get_bootstrap_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    exe_dir.join("bootstrap.enc")
}

pub fn load_bootstrap_config() -> BootstrapConfig {
    let path = get_bootstrap_config_path();
    if let Some((config, _)) = load_secure_data::<BootstrapConfig>(&path) {
        return config;
    }
    BootstrapConfig::default()
}

pub fn save_bootstrap_config(config: &BootstrapConfig) -> bool {
    let path = get_bootstrap_config_path();
    save_secure_data(&path, config)
}

/// Repository for config persistence.
pub struct ConfigRepository {
    save_path: PathBuf,
    engines_path: PathBuf,
    profiles_dir: PathBuf,
}

impl ConfigRepository {
    pub fn new(base_dir: PathBuf) -> Self {
        let save_path = base_dir.join("save.enc");
        let engines_path = base_dir.join("engines.enc");
        let profiles_dir = base_dir.join("profiles");
        fs::create_dir_all(&profiles_dir).ok();
        let repo = ConfigRepository {
            save_path,
            engines_path,
            profiles_dir,
        };
        repo.populate_defaults_if_missing();
        repo
    }

    pub fn get_save_path(&self) -> PathBuf {
        self.save_path.clone()
    }

    pub fn get_engines_path(&self) -> PathBuf {
        self.engines_path.clone()
    }

    pub fn get_profiles_dir(&self) -> PathBuf {
        self.profiles_dir.clone()
    }

    pub fn load_engines(&self) -> EnginePresets {
        if let Some((presets, encrypted)) = load_secure_data::<EnginePresets>(&self.engines_path) {
            if !encrypted {
                save_secure_data(&self.engines_path, &presets);
            }
            return presets;
        }
        EnginePresets::default()
    }

    pub fn save_engines(&self, presets: &EnginePresets) {
        save_secure_data(&self.engines_path, presets);
    }

    pub fn load_config(&self) -> SavedConfig {
        if let Some((config, encrypted)) = load_secure_data::<SavedConfig>(&self.save_path) {
            if !encrypted {
                save_secure_data(&self.save_path, &config);
            }
            return config;
        }
        SavedConfig {
            langue: "English".to_string(),
            moteur: "LM Studio".to_string(),
            url_api: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
            nom_modele: "qwen/qwen3.6-35b-a3b".to_string(),
            ..SavedConfig::default()
        }
    }

    pub fn save_config(&self, config: &SavedConfig) {
        save_secure_data(&self.save_path, config);
    }

    pub fn list_profiles(&self) -> Vec<String> {
        if let Ok(entries) = fs::read_dir(&self.profiles_dir) {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.strip_suffix(".enc"))
                        .map(String::from)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn load_profile(&self, name: &str) -> Option<SavedConfig> {
        let path = self.profiles_dir.join(format!("{}.enc", name));
        if path.exists() {
            if let Some((config, encrypted)) = load_secure_data::<SavedConfig>(&path) {
                if !encrypted {
                    save_secure_data(&path, &config);
                }
                return Some(config);
            }
        }
        None
    }

    pub fn save_profile(&self, name: &str, config: &SavedConfig) -> bool {
        let path = self.profiles_dir.join(format!("{}.enc", name));
        if fs::create_dir_all(&self.profiles_dir).is_ok() {
            return save_secure_data(&path, config);
        }
        false
    }

    pub fn delete_profile(&self, name: &str) -> bool {
        let path = self.profiles_dir.join(format!("{}.enc", name));
        fs::remove_file(&path).is_ok()
    }

    pub fn populate_defaults_if_missing(&self) {
        // 1. If save.enc doesn't exist, create it from default config
        if !self.save_path.exists() {
            let default_config = crate::defaults::DefaultProfileFactory::build_default_config();
            self.save_config(&default_config);
        }

        // 2. If engines.enc doesn't exist, create it from default presets
        if !self.engines_path.exists() {
            let default_presets = EnginePresets::default();
            self.save_engines(&default_presets);
        }

        // 3. If profiles directory doesn't have profiles, write the default profiles
        let list = self.list_profiles();
        if list.is_empty() {
            let kilo_profile = crate::defaults::DefaultProfileFactory::build_kilo_vscode_profile();
            self.save_profile("Default_Kilo_VSCode", &kilo_profile);

            let gravity_profile = crate::defaults::DefaultProfileFactory::build_gravity_pipeline_profile();
            self.save_profile("Template_Gravity_Pipeline", &gravity_profile);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn cleanup(name: &str) {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_engines_defaults() {
        let dir = temp_dir("engines");
        let repo = ConfigRepository::new(dir.clone());
        let presets = repo.load_engines();
        assert!(presets.lm_studio.modeles.len() > 0);
        assert!(presets.ollama.modeles.len() > 0);
        cleanup("engines");
    }

    #[test]
    fn test_save_and_load_engines() {
        let dir = temp_dir("engines2");
        let repo = ConfigRepository::new(dir.clone());
        let presets = EnginePresets::default();
        repo.save_engines(&presets);
        let loaded = repo.load_engines();
        assert_eq!(loaded.lm_studio.url, "http://127.0.0.1:1234/v1/chat/completions");
        cleanup("engines2");
    }

    #[test]
    fn test_load_config_defaults() {
        let dir = temp_dir("config");
        let repo = ConfigRepository::new(dir.clone());
        let config = repo.load_config();
        assert_eq!(config.langue, "English");
        cleanup("config");
    }

    #[test]
    fn test_save_and_load_config() {
        let dir = temp_dir("config2");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            contexte: "test context".to_string(),
            objectif: "test objective".to_string(),
            ..SavedConfig::default()
        };
        repo.save_config(&config);
        let loaded = repo.load_config();
        assert_eq!(loaded.contexte, "test context");
        cleanup("config2");
    }

    #[test]
    fn test_save_and_load_profile() {
        let dir = temp_dir("profile");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            contexte: "profile context".to_string(),
            ..SavedConfig::default()
        };
        assert!(repo.save_profile("test_profile", &config));
        let loaded = repo.load_profile("test_profile");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().contexte, "profile context");
        cleanup("profile");
    }

    #[test]
    fn test_list_profiles() {
        let dir = temp_dir("list_profiles_unit");
        let repo = ConfigRepository::new(dir.clone());
        let profiles_dir = dir.join("profiles");
        let _ = fs::create_dir_all(&profiles_dir);
        fs::write(profiles_dir.join("profile1.enc"), "{}").ok();
        fs::write(profiles_dir.join("profile2.enc"), "{}").ok();
        let profiles = repo.list_profiles();
        assert!(profiles.contains(&"profile1".to_string()));
        assert!(profiles.contains(&"profile2".to_string()));
        cleanup("list_profiles_unit");
    }

    #[test]
    fn test_delete_profile() {
        let dir = temp_dir("delete_profile_unit");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig::default();
        repo.save_profile("to_delete", &config);
        assert!(repo.delete_profile("to_delete"));
        assert!(!repo.delete_profile("to_delete"));
        cleanup("delete_profile_unit");
    }
}
