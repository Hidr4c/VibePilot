use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::memory::TaskGraph;
use super::helpers::{compress_blob, decompress_blob};

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

/// Engine profile with dynamic key support.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineProfile {
    pub url: String,
    pub modeles: Vec<String>,
}

/// Default engine presets (LM Studio, Ollama, Custom) + dynamic custom engines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub decomposer_taches: bool,
    pub activer_reflexion: bool,
    pub max_tentatives_par_tache: u32,
    pub pipeline_vision_avance: bool,
    pub activer_systeme_fast_slow: bool,
    pub activer_compression_historique: bool,
    pub economie_ecriture_ssd: bool,
    pub garder_ecran_actif: bool,
    pub step_mode_enabled: bool,
    pub undo_enabled: bool,
    pub allow_clipboard_read_without_confirm: bool,
    pub activer_recadrage_workspace: bool,
    pub dossier_sauvegarde_captures: Option<String>,
    pub webhook_url: Option<String>,
    pub activer_roi: bool,
    pub roi_x: i32,
    pub roi_y: i32,
    pub roi_width: i32,
    pub roi_height: i32,
    pub chiffrement_dpapi: bool,
    pub trace_actions_visuelles: bool,
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
            decomposer_taches: false,
            activer_reflexion: false,
            max_tentatives_par_tache: 5,
            pipeline_vision_avance: false,
            activer_systeme_fast_slow: false,
            activer_compression_historique: false,
            economie_ecriture_ssd: true,
            garder_ecran_actif: true,
            step_mode_enabled: false,
            undo_enabled: false,
            allow_clipboard_read_without_confirm: false,
            activer_recadrage_workspace: true,
            dossier_sauvegarde_captures: None,
            webhook_url: None,
            activer_roi: false,
            roi_x: 0,
            roi_y: 0,
            roi_width: 1920,
            roi_height: 1080,
            chiffrement_dpapi: false,
            trace_actions_visuelles: false,
        }
    }
}

/// Long-term memory storage structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LongTermMemory {
    pub entries: Vec<serde_json::Value>,
}

/// Snapshot of past session details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    pub timestamp: String,
    pub objective: String,
}

/// Unified persistence state struct containing all application data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentState {
    pub config: SavedConfig,
    pub engines: EnginePresets,
    
    #[serde(serialize_with = "compress_blob", deserialize_with = "decompress_blob")]
    pub profiles: HashMap<String, SavedConfig>,
    
    #[serde(serialize_with = "compress_blob", deserialize_with = "decompress_blob")]
    pub long_term_memory: LongTermMemory,
    
    pub session_snapshots: Vec<SessionSnapshot>,
    
    #[serde(serialize_with = "compress_blob", deserialize_with = "decompress_blob")]
    pub task_graph: Option<TaskGraph>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            config: SavedConfig::default(),
            engines: EnginePresets::default(),
            profiles: HashMap::new(),
            long_term_memory: LongTermMemory { entries: Vec::new() },
            session_snapshots: Vec::new(),
            task_graph: None,
        }
    }
}

/// Bootstrap configuration for storing the custom storage directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapConfig {
    pub storage_dir: Option<String>,
}

impl crate::secure_store::DpapiConfigurable for PersistentState {
    fn is_dpapi_enabled(&self) -> bool {
        self.config.chiffrement_dpapi
    }
}
