//! Storage migration service.
//!
//! Handles moving all persistent data (encrypted files, profiles, engines)
//! from one directory to another.

use std::fs;
use std::path::PathBuf;
use crate::config::{ConfigurationRepository, SavedConfig, EnginePresets};

pub struct StorageMigrator;

impl StorageMigrator {
    pub fn migrate(
        new_path: String,
        old_config_repo: &dyn ConfigurationRepository,
    ) -> Result<(PathBuf, SavedConfig, EnginePresets), String> {
        let new_base = PathBuf::from(&new_path);
        if let Err(e) = fs::create_dir_all(&new_base) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Save bootstrap config
        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(new_path.clone());
        crate::config::save_bootstrap_config(&bootstrap);

        // Retrieve old paths from current config_repo
        let old_save = old_config_repo.get_save_path();
        let old_engines = old_config_repo.get_engines_path();
        let old_profiles = old_config_repo.get_profiles_dir();
        let old_store = old_config_repo.get_store_path();
        let old_key = old_config_repo.get_key_path();

        // Define new paths
        let new_save = new_base.join("save.enc");
        let new_engines = new_base.join("engines.enc");
        let new_profiles = new_base.join("profiles");
        let new_store = new_base.join("vibepilot_data.enc");
        let new_key = new_base.join("key.enc");

        // Copy vibepilot_data.enc
        if old_store.exists() {
            let _ = fs::copy(&old_store, &new_store);
        }
        // Copy key.enc
        if old_key.exists() {
            let _ = fs::copy(&old_key, &new_key);
        }
        // Copy save.enc
        if old_save.exists() {
            let _ = fs::copy(&old_save, &new_save);
        }
        // Copy engines.enc
        if old_engines.exists() {
            let _ = fs::copy(&old_engines, &new_engines);
        }
        // Copy profiles
        if old_profiles.exists() {
            let _ = fs::create_dir_all(&new_profiles);
            if let Ok(entries) = fs::read_dir(&old_profiles) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name() {
                            let _ = fs::copy(&path, new_profiles.join(name));
                        }
                    }
                }
            }
        }

        Ok((new_base, SavedConfig::default(), EnginePresets::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigRepository;
    use crate::common::{temp_dir, cleanup};

    #[test]
    fn test_storage_migrator() {
        let src_path = temp_dir("service_migrator_src");
        let dst_path = temp_dir("service_migrator_dst");

        let src_repo = ConfigRepository::new(src_path.clone());
        
        let key_file = src_path.join("key.enc");
        let _ = std::fs::write(&key_file, b"secret");
        
        let store_file = src_path.join("vibepilot_data.enc");
        let _ = std::fs::write(&store_file, b"data");

        let save_file = src_path.join("save.enc");
        let _ = std::fs::write(&save_file, b"save");

        let engines_file = src_path.join("engines.enc");
        let _ = std::fs::write(&engines_file, b"engines");

        let profiles_dir = src_path.join("profiles");
        let _ = std::fs::create_dir_all(&profiles_dir);
        let profile_file = profiles_dir.join("default.toml");
        let _ = std::fs::write(&profile_file, b"profile");

        let res = StorageMigrator::migrate(
            dst_path.to_string_lossy().to_string(),
            &src_repo,
        );

        assert!(res.is_ok());
        let (new_base, _, _) = res.unwrap();
        assert_eq!(new_base, dst_path);
        
        assert!(dst_path.join("key.enc").exists());
        assert!(dst_path.join("vibepilot_data.enc").exists());
        assert!(dst_path.join("save.enc").exists());
        assert!(dst_path.join("engines.enc").exists());
        assert!(dst_path.join("profiles/default.toml").exists());

        cleanup("service_migrator_src");
        cleanup("service_migrator_dst");
    }

    #[test]
    fn test_storage_migrator_invalid_path() {
        let src_path = temp_dir("service_migrator_src_invalid");
        let src_repo = ConfigRepository::new(src_path.clone());
        
        // Pass a directory path containing invalid characters on Windows to trigger an error
        let res = StorageMigrator::migrate(
            "C:\\invalid|dir:*?\\<>".to_string(),
            &src_repo,
        );
        assert!(res.is_err());
        
        cleanup("service_migrator_src_invalid");
    }
}
