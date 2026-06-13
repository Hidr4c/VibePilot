use std::path::PathBuf;
use std::fs;
use crate::memory::TaskGraph;
use super::models::{PersistentState, SavedConfig, EnginePresets};
use super::helpers::load_secure_data;
use super::{ConfigManagement, ProfileManagement, EngineManagement, StateManagement, ConfigPaths};

pub struct ConfigRepository {
    save_path: PathBuf,
    engines_path: PathBuf,
    profiles_dir: PathBuf,
    base_dir: PathBuf,
    pub store: crate::secure_store::SecureStore<PersistentState>,
}

impl ConfigRepository {
    pub fn new(base_dir: PathBuf) -> Self {
        let save_path = base_dir.join("save.enc");
        let engines_path = base_dir.join("engines.enc");
        let profiles_dir = base_dir.join("profiles");
        fs::create_dir_all(&profiles_dir).ok();

        let store_path = base_dir.join("vibepilot_data.enc");

        let mut initial_state = PersistentState::default();
        let mut performed_migration = false;

        if !store_path.exists() {
            let mut has_old_data = false;
            if save_path.exists() {
                if let Some((config, _)) = load_secure_data::<SavedConfig>(&save_path) {
                    initial_state.config = config;
                    has_old_data = true;
                }
            }
            if engines_path.exists() {
                if let Some((engines, _)) = load_secure_data::<EnginePresets>(&engines_path) {
                    initial_state.engines = engines;
                    has_old_data = true;
                }
            }
            if profiles_dir.exists() {
                if let Ok(entries) = fs::read_dir(&profiles_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("enc") {
                            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                                if let Some((profile_config, _)) = load_secure_data::<SavedConfig>(&path) {
                                    initial_state.profiles.insert(name.to_string(), profile_config);
                                    has_old_data = true;
                                }
                            }
                        }
                    }
                }
            }

            if has_old_data {
                performed_migration = true;
            }
        }

        let interval_secs = if initial_state.config.economie_ecriture_ssd { 300 } else { 30 };

        let store = crate::secure_store::SecureStore::new(store_path, initial_state, interval_secs)
            .expect("Failed to initialize secure store");

        let repo = ConfigRepository {
            save_path,
            engines_path,
            profiles_dir,
            base_dir,
            store,
        };

        repo.populate_defaults_if_missing();

        if performed_migration {
            let old_save_path = repo.save_path.clone();
            let old_engines_path = repo.engines_path.clone();
            let old_profiles_dir = repo.profiles_dir.clone();

            if old_save_path.exists() {
                let _ = fs::rename(&old_save_path, old_save_path.with_extension("enc.bak"));
            }
            if old_engines_path.exists() {
                let _ = fs::rename(&old_engines_path, old_engines_path.with_extension("enc.bak"));
            }
            if old_profiles_dir.exists() {
                if let Ok(entries) = fs::read_dir(&old_profiles_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("enc") {
                            let _ = fs::rename(&path, path.with_extension("enc.bak"));
                        }
                    }
                }
            }
            let _ = repo.store.save_now();
        }

        repo
    }



    fn populate_defaults_if_missing(&self) {
        let mut updated = false;
        self.store.update(|state| {
            if state.profiles.is_empty() {
                let kilo_profile = crate::defaults::DefaultProfileFactory::build_kilo_vscode_profile();
                state.profiles.insert("Default_Kilo_VSCode".to_string(), kilo_profile);

                let gravity_profile = crate::defaults::DefaultProfileFactory::build_gravity_pipeline_profile();
                state.profiles.insert("Template_Gravity_Pipeline".to_string(), gravity_profile);
                updated = true;
            }
        });
        if updated {
            let _ = self.store.save_now();
        }
    }


}

impl ConfigManagement for ConfigRepository {
    fn load_config(&self) -> SavedConfig {
        self.store.read(|state| state.config.clone())
    }

    fn save_config(&self, config: &SavedConfig) {
        self.store.update(|state| {
            state.config = config.clone();
        });
        let interval_secs = if config.economie_ecriture_ssd { 300 } else { 30 };
        self.store.set_interval(std::time::Duration::from_secs(interval_secs));
    }

    fn save_now(&self) -> Result<(), String> {
        self.store.save_now()
    }

    fn get_save_path(&self) -> PathBuf {
        self.save_path.clone()
    }

    fn get_key_path(&self) -> PathBuf {
        self.base_dir.join("key.enc")
    }
}

impl ProfileManagement for ConfigRepository {
    fn load_profile(&self, name: &str) -> Option<SavedConfig> {
        self.store.read(|state| state.profiles.get(name).cloned())
    }

    fn save_profile(&self, name: &str, config: &SavedConfig) -> bool {
        self.store.update(|state| {
            state.profiles.insert(name.to_string(), config.clone());
        });
        true
    }

    fn delete_profile(&self, name: &str) -> bool {
        let mut deleted = false;
        self.store.update(|state| {
            if state.profiles.remove(name).is_some() {
                deleted = true;
            }
        });
        deleted
    }

    fn list_profiles(&self) -> Vec<String> {
        self.store.read(|state| state.profiles.keys().cloned().collect())
    }

    fn get_profiles_dir(&self) -> PathBuf {
        self.profiles_dir.clone()
    }

    fn export_profiles(&self, path: &std::path::Path) -> Result<(), String> {
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

    fn import_profiles(&self, path: &std::path::Path) -> Result<usize, String> {
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

    fn export_single_profile(&self, name: &str, path: &std::path::Path) -> Result<(), String> {
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

    fn import_single_profile(&self, path: &std::path::Path) -> Result<String, String> {
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
}

impl EngineManagement for ConfigRepository {
    fn load_engines(&self) -> EnginePresets {
        self.store.read(|state| state.engines.clone())
    }

    fn save_engines(&self, presets: &EnginePresets) {
        self.store.update(|state| {
            state.engines = presets.clone();
        });
    }

    fn get_engines_path(&self) -> PathBuf {
        self.engines_path.clone()
    }

    fn export_engines(&self, path: &std::path::Path) -> Result<(), String> {
        let presets = self.load_engines();
        let output = serde_json::to_string_pretty(&presets)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        std::fs::write(path, &output)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        Ok(())
    }

    fn import_engines(&self, path: &std::path::Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let presets: EnginePresets = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;
        self.save_engines(&presets);
        Ok(())
    }

    fn export_single_engine(&self, name: &str, path: &std::path::Path) -> Result<(), String> {
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

    fn import_single_engine(&self, path: &std::path::Path) -> Result<String, String> {
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file path".to_string())?
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let profile: super::models::EngineProfile = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;

        let mut presets = self.load_engines();
        presets.insert(file_stem.clone(), profile);
        self.save_engines(&presets);

        Ok(file_stem)
    }
}

impl StateManagement for ConfigRepository {
    fn load_task_graph(&self) -> Option<TaskGraph> {
        self.store.read(|state| state.task_graph.clone())
    }

    fn save_task_graph(&self, graph: Option<TaskGraph>) {
        self.store.update(|state| {
            state.task_graph = graph.clone();
        });
    }

    fn export_config(&self, path: &std::path::Path) -> Result<(), String> {
        let config = self.load_config();
        let output = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        std::fs::write(path, &output)
            .map_err(|e| format!("Failed to write export file: {}", e))?;
        Ok(())
    }

    fn import_config(&self, path: &std::path::Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read import file: {}", e))?;
        let config: SavedConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;
        self.save_config(&config);
        Ok(())
    }
}

impl ConfigPaths for ConfigRepository {
    fn get_base_dir(&self) -> PathBuf {
        self.base_dir.clone()
    }

    fn get_store_path(&self) -> PathBuf {
        self.base_dir.join("vibepilot_data.enc")
    }
}
