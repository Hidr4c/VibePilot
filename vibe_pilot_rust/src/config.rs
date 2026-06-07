//! Configuration persistence for VibePilot.
//!
//! Handles encrypted configuration storage using Windows DPAPI,
//! profile management, and bootstrap configuration.
//!
//! # Security
//!
//! On Windows, configuration files are encrypted using DPAPI (Data Protection API),
//! which ties decryption to the current Windows user account. On non-Windows platforms,
//! encryption is a no-op and data is stored in plain text as a fallback.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::io::Write;

/// Thread-safe persistent logger for VibePilot actions.
///
/// Writes log entries to a file in the storage directory with timestamps.
/// Does not include screenshots or sensitive data.
pub struct ActionLogger {
    writer: Mutex<fs::File>,
}

impl ActionLogger {
    /// Creates a new logger writing to the given path.
    pub fn new(log_path: PathBuf) -> Self {
        let file = if log_path.exists() {
            fs::OpenOptions::new().append(true).open(&log_path).unwrap_or_else(|_| {
                fs::File::create(&log_path).expect("Failed to create log file")
            })
        } else {
            fs::File::create(&log_path).expect("Failed to create log file")
        };
        Self {
            writer: Mutex::new(file),
        }
    }

    /// Writes a log entry with timestamp.
    pub fn log(&self, action: &str, detail: &str, result: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] Action: {} | Details: {} | Result: {}\n", timestamp, action, detail, result);

        if let Ok(mut file) = self.writer.lock() {
            let _ = file.write_all(entry.as_bytes());
            let _ = file.flush();
        }
    }

    /// Writes an info-level log entry.
    pub fn info(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] INFO: {}\n", timestamp, message);
        if let Ok(mut file) = self.writer.lock() {
            let _ = file.write_all(entry.as_bytes());
            let _ = file.flush();
        }
    }

    /// Writes an error-level log entry.
    pub fn error(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] ERROR: {}\n", timestamp, message);
        if let Ok(mut file) = self.writer.lock() {
            let _ = file.write_all(entry.as_bytes());
            let _ = file.flush();
        }
    }
}

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

    /// Encrypts data using Windows DPAPI.
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

    /// Decrypts data previously encrypted with DPAPI.
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
    /// No-op encryption on non-Windows platforms.
    pub fn encrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
    /// No-op decryption on non-Windows platforms.
    pub fn decrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// Loads and decrypts a secure data file.
///
/// Returns `Some((data, encrypted))` if the file exists and can be parsed,
/// where `encrypted` indicates whether the data was decrypted from DPAPI.
/// Falls back to plain TOML parsing if DPAPI decryption fails.
///
/// # Safety
///
/// If the file is corrupted, this function silently falls back to plain text
/// parsing. A corrupted encrypted file will be re-encrypted on next save.
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

/// Saves data with DPAPI encryption if available, falling back to plain text.
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
    /// Returns all engine preset keys including custom engines.
    pub fn all_keys(&self) -> Vec<String> {
        let mut keys = vec!["LM Studio".to_string(), "Ollama".to_string(), "Perso / Autre".to_string()];
        keys.extend(self.custom_engines.keys().cloned());
        keys
    }

    /// Gets an engine profile by name.
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

    /// Inserts a custom engine profile.
    pub fn insert(&mut self, name: String, profile: EngineProfile) {
        self.custom_engines.insert(name, profile);
    }

    /// Removes a custom engine profile by name.
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

/// Default configuration stored in `save.enc`.
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

    // LLM Cloud Credentials & Authentication
    pub auth_mode: String,       // "none", "api_key", "basic_auth"
    pub auth_api_key: String,
    pub auth_login: String,
    pub auth_password: String,

    // Configurable timeout (in seconds)
    pub request_timeout_secs: u64,

    // Verification and activity detection
    pub verifier_placement_souris: bool,
    pub detecter_activite_utilisateur: bool,

    // Last loaded profile name
    pub dernier_profil: String,

    // Dedicated Vision LLM settings
    pub utiliser_moteur_vision_dedie: bool,
    pub moteur_vision: String,
    pub url_api_vision: String,
    pub nom_modele_vision: String,
    pub auth_mode_vision: String,
    pub auth_api_key_vision: String,
    pub auth_login_vision: String,
    pub auth_password_vision: String,
    pub request_timeout_secs_vision: u64,
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

            // Auth defaults
            auth_mode: "none".to_string(),
            auth_api_key: "".to_string(),
            auth_login: "".to_string(),
            auth_password: "".to_string(),

            // Timeout default (120 seconds)
            request_timeout_secs: 120,

            // Verification & detection defaults
            verifier_placement_souris: false,
            detecter_activite_utilisateur: false,

            // Last loaded profile default
            dernier_profil: "".to_string(),

            // Dedicated Vision LLM defaults
            utiliser_moteur_vision_dedie: false,
            moteur_vision: ENGINE_LM_STUDIO.to_string(),
            url_api_vision: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
            nom_modele_vision: "qwen2.5-vl-7b-instruct".to_string(),
            auth_mode_vision: "none".to_string(),
            auth_api_key_vision: "".to_string(),
            auth_login_vision: "".to_string(),
            auth_password_vision: "".to_string(),
            request_timeout_secs_vision: 120,
        }
    }
}

/// Checks if a string contains any dangerous terminal command keywords.
///
/// This is a heuristic check and not a complete safety guarantee.
/// It covers common destructive commands across Windows and Unix systems.
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

/// Bootstrap configuration for storing the custom storage directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapConfig {
    pub storage_dir: Option<String>,
}

/// Returns the path to the bootstrap configuration file.
pub fn get_bootstrap_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    exe_dir.join("bootstrap.enc")
}

/// Loads the bootstrap configuration, returning defaults if the file doesn't exist.
pub fn load_bootstrap_config() -> BootstrapConfig {
    let path = get_bootstrap_config_path();
    if let Some((config, _)) = load_secure_data::<BootstrapConfig>(&path) {
        return config;
    }
    BootstrapConfig::default()
}

/// Saves the bootstrap configuration.
pub fn save_bootstrap_config(config: &BootstrapConfig) -> bool {
    let path = get_bootstrap_config_path();
    save_secure_data(&path, config)
}

/// Repository for config persistence.
///
/// Manages storage paths for `save.enc`, `engines.enc`, and the profiles directory.
/// All files are encrypted with DPAPI on Windows.
pub struct ConfigRepository {
    save_path: PathBuf,
    engines_path: PathBuf,
    profiles_dir: PathBuf,
}

impl ConfigRepository {
    /// Creates a new config repository with the given base directory.
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

    /// Returns the path to the save.enc file.
    pub fn get_save_path(&self) -> PathBuf {
        self.save_path.clone()
    }

    /// Returns the path to the engines.enc file.
    pub fn get_engines_path(&self) -> PathBuf {
        self.engines_path.clone()
    }

    /// Returns the path to the profiles directory.
    pub fn get_profiles_dir(&self) -> PathBuf {
        self.profiles_dir.clone()
    }

    /// Loads engine presets from the encrypted engines file.
    /// Returns defaults if the file doesn't exist or can't be decrypted.
    pub fn load_engines(&self) -> EnginePresets {
        if let Some((presets, encrypted)) = load_secure_data::<EnginePresets>(&self.engines_path) {
            if !encrypted {
                save_secure_data(&self.engines_path, &presets);
            }
            return presets;
        }
        EnginePresets::default()
    }

    /// Saves engine presets to the encrypted engines file.
    pub fn save_engines(&self, presets: &EnginePresets) {
        save_secure_data(&self.engines_path, presets);
    }

    /// Loads the main configuration from save.enc.
    /// Returns a config with sensible defaults if the file doesn't exist.
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

    /// Saves the main configuration to save.enc with DPAPI encryption.
    pub fn save_config(&self, config: &SavedConfig) {
        save_secure_data(&self.save_path, config);
    }

    /// Lists all profile names (without .enc extension).
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

    /// Loads a profile by name. Returns `None` if not found or can't be decrypted.
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

    /// Saves a profile by name to an encrypted file.
    pub fn save_profile(&self, name: &str, config: &SavedConfig) -> bool {
        let path = self.profiles_dir.join(format!("{}.enc", name));
        if fs::create_dir_all(&self.profiles_dir).is_ok() {
            return save_secure_data(&path, config);
        }
        false
    }

    /// Deletes a profile by name.
    pub fn delete_profile(&self, name: &str) -> bool {
        let path = self.profiles_dir.join(format!("{}.enc", name));
        fs::remove_file(&path).is_ok()
    }

    /// Populates default save.enc, engines.enc, and profile files if missing.
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

    // === Import/Export Methods ===

    /// Exports all profiles to a JSON file at the given path.
    /// The JSON file is human-readable and can be shared between machines.
    pub fn export_profiles(&self, path: &std::path::Path) -> Result<(), String> {
        let profiles = self.list_profiles();
        let mut map = serde_json::Map::new();

        for name in &profiles {
            if let Some(config) = self.load_profile(name) {
                let json = serde_json::to_value(&config)
                    .map_err(|e| format!("Failed to serialize profile '{}': {}", name, e))?;
                map.insert(name.clone(), json);
            }
        }

        let output = serde_json::to_string_pretty(&map)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        std::fs::write(path, &output)
            .map_err(|e| format!("Failed to write export file: {}", e))?;

        Ok(())
    }

    /// Imports profiles from a JSON file.
    /// Existing profiles with the same name will be overwritten.
    pub fn import_profiles(&self, path: &std::path::Path) -> Result<usize, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;

        let mut count = 0;
        for (name, value) in &map {
            let config: SavedConfig = serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to deserialize profile '{}': {}", name, e))?;
            if self.save_profile(name, &config) {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Exports a single profile to a JSON file at the given path.
    pub fn export_single_profile(&self, name: &str, path: &std::path::Path) -> Result<(), String> {
        if let Some(config) = self.load_profile(name) {
            let output = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize profile: {}", e))?;
            std::fs::write(path, &output)
                .map_err(|e| format!("Failed to write export file: {}", e))?;
            Ok(())
        } else {
            Err(format!("Profile '{}' not found", name))
        }
    }

    /// Imports a single profile from a JSON file.
    /// Returns the imported profile name.
    pub fn import_single_profile(&self, path: &std::path::Path) -> Result<String, String> {
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file path".to_string())?
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let config: SavedConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;

        if self.save_profile(&file_stem, &config) {
            Ok(file_stem)
        } else {
            Err(format!("Failed to save profile '{}'", file_stem))
        }
    }

    /// Exports engine presets to a JSON file at the given path.
    pub fn export_engines(&self, path: &std::path::Path) -> Result<(), String> {
        let presets = self.load_engines();
        let output = serde_json::to_string_pretty(&presets)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        std::fs::write(path, &output)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        Ok(())
    }

    /// Imports engine presets from a JSON file.
    pub fn import_engines(&self, path: &std::path::Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let presets: EnginePresets = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;
        self.save_engines(&presets);
        Ok(())
    }

    /// Exports a single engine profile to a JSON file at the given path.
    pub fn export_single_engine(&self, name: &str, path: &std::path::Path) -> Result<(), String> {
        let presets = self.load_engines();
        if let Some(profile) = presets.get(name) {
            let output = serde_json::to_string_pretty(&profile)
                .map_err(|e| format!("Failed to serialize engine profile: {}", e))?;
            std::fs::write(path, &output)
                .map_err(|e| format!("Failed to write export file: {}", e))?;
            Ok(())
        } else {
            Err(format!("Engine preset '{}' not found", name))
        }
    }

    /// Imports a single engine profile from a JSON file.
    /// Returns the imported engine name.
    pub fn import_single_engine(&self, path: &std::path::Path) -> Result<String, String> {
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file path".to_string())?
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let profile: EngineProfile = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;

        let mut presets = self.load_engines();
        presets.insert(file_stem.clone(), profile);
        self.save_engines(&presets);

        Ok(file_stem)
    }

    /// Exports the current main config (save.enc) to a JSON file.
    pub fn export_config(&self, path: &std::path::Path) -> Result<(), String> {
        let config = self.load_config();
        let output = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        std::fs::write(path, &output)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        Ok(())
    }

    /// Imports the main config from a JSON file (overwrites save.enc).
    pub fn import_config(&self, path: &std::path::Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let config: SavedConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;
        self.save_config(&config);
        Ok(())
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

    #[test]
    fn test_export_import_single_profile() {
        let dir = temp_dir("single_profile_export_import");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            contexte: "single export test context".to_string(),
            ..SavedConfig::default()
        };
        repo.save_profile("my_special_profile", &config);

        let export_path = dir.join("my_special_profile.json");
        assert!(repo.export_single_profile("my_special_profile", &export_path).is_ok());
        assert!(export_path.exists());

        // Now import it under a different name
        let import_path = dir.join("imported_special_profile.json");
        std::fs::copy(&export_path, &import_path).unwrap();
        
        let imported_name = repo.import_single_profile(&import_path).unwrap();
        assert_eq!(imported_name, "imported_special_profile");

        let loaded = repo.load_profile("imported_special_profile").unwrap();
        assert_eq!(loaded.contexte, "single export test context");

        cleanup("single_profile_export_import");
    }

    #[test]
    fn test_export_import_single_engine() {
        let dir = temp_dir("single_engine_export_import");
        let repo = ConfigRepository::new(dir.clone());
        let presets = repo.load_engines();
        
        let export_path = dir.join("LM Studio Export.json");
        assert!(repo.export_single_engine("LM Studio", &export_path).is_ok());
        assert!(export_path.exists());

        // Import it under a custom engine name
        let import_path = dir.join("My Custom Imported Engine.json");
        std::fs::copy(&export_path, &import_path).unwrap();

        let imported_name = repo.import_single_engine(&import_path).unwrap();
        assert_eq!(imported_name, "My Custom Imported Engine");

        // Reload presets and verify
        let updated_presets = repo.load_engines();
        let imported_profile = updated_presets.get("My Custom Imported Engine").unwrap();
        assert_eq!(imported_profile.url, presets.get("LM Studio").unwrap().url);

        cleanup("single_engine_export_import");
    }
}
