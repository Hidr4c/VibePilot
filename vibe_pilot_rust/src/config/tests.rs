use super::*;
use std::fs;

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
    assert!(!presets.lm_studio.modeles.is_empty());
    assert!(!presets.ollama.modeles.is_empty());
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
    let config = SavedConfig::default();
    repo.save_profile("profile1", &config);
    repo.save_profile("profile2", &config);
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

#[test]
fn test_selective_compression_serialization() {
    let mut state = PersistentState::default();
    let mut config1 = SavedConfig::default();
    config1.contexte = "Selective Compression test profile content".to_string();
    state.profiles.insert("ProfileCompress".to_string(), config1);

    let serialized = rmp_serde::to_vec(&state).unwrap();
    
    let deserialized: PersistentState = rmp_serde::from_slice(&serialized).unwrap();
    assert_eq!(deserialized.profiles.get("ProfileCompress").unwrap().contexte, "Selective Compression test profile content");
}

#[test]
fn test_save_and_load_task_graph() {
    let dir = temp_dir("task_graph_unit");
    let repo = ConfigRepository::new(dir.clone());
    
    // Load initially - should be None
    let initial = repo.load_task_graph();
    assert!(initial.is_none());

    // Create a mock task graph
    let mut graph = crate::memory::TaskGraph::new();
    let task = crate::memory::TaskNode::new(
        crate::memory::TaskId(1),
        "Test subtask description".to_string(),
        vec![]
    );
    graph.add_task(task);

    // Save
    repo.save_task_graph(Some(graph.clone()));

    // Load back
    let loaded = repo.load_task_graph();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), graph);

    // Save None
    repo.save_task_graph(None);
    let loaded_none = repo.load_task_graph();
    assert!(loaded_none.is_none());

    cleanup("task_graph_unit");
}

#[test]
fn test_config_repository_getters_and_exports() {
    let dir = temp_dir("repo_getters");
    let repo = ConfigRepository::new(dir.clone());

    assert_eq!(repo.get_base_dir(), dir);
    assert_eq!(repo.get_save_path(), dir.join("save.enc"));
    assert_eq!(repo.get_engines_path(), dir.join("engines.enc"));
    assert_eq!(repo.get_profiles_dir(), dir.join("profiles"));
    assert_eq!(repo.get_store_path(), dir.join("vibepilot_data.enc"));
    assert_eq!(repo.get_key_path(), dir.join("key.enc"));

    // Save and export config
    let config = SavedConfig {
        contexte: "export test context".to_string(),
        ..SavedConfig::default()
    };
    repo.save_config(&config);
    let cfg_path = dir.join("setup_export.json");
    assert!(repo.export_config(&cfg_path).is_ok());
    assert!(cfg_path.exists());

    // Import config
    let imported_cfg_path = dir.join("setup_import.json");
    std::fs::copy(&cfg_path, &imported_cfg_path).unwrap();
    assert!(repo.import_config(&imported_cfg_path).is_ok());

    // Export profiles
    let profiles_path = dir.join("profiles_export.json");
    assert!(repo.export_profiles(&profiles_path).is_ok());
    assert!(profiles_path.exists());

    // Import profiles
    let imported_profiles_path = dir.join("profiles_import.json");
    std::fs::copy(&profiles_path, &imported_profiles_path).unwrap();
    assert!(repo.import_profiles(&imported_profiles_path).is_ok());

    // Export engines
    let engines_path = dir.join("engines_export.json");
    assert!(repo.export_engines(&engines_path).is_ok());
    assert!(engines_path.exists());

    // Import engines
    let imported_engines_path = dir.join("engines_import.json");
    std::fs::copy(&engines_path, &imported_engines_path).unwrap();
    assert!(repo.import_engines(&imported_engines_path).is_ok());

    // save_now
    assert!(repo.save_now().is_ok());

    cleanup("repo_getters");
}

#[test]
fn test_config_repository_error_cases() {
    let dir = temp_dir("repo_errors");
    let repo = ConfigRepository::new(dir.clone());

    // 1. Export non-existent profile
    let path_p = dir.join("non_existent_profile.json");
    let res_p = repo.export_single_profile("NonExistent", &path_p);
    assert!(res_p.is_err());
    assert!(res_p.unwrap_err().contains("not found"));

    // 2. Export non-existent engine
    let path_e = dir.join("non_existent_engine.json");
    let res_e = repo.export_single_engine("NonExistent", &path_e);
    assert!(res_e.is_err());
    assert!(res_e.unwrap_err().contains("not found"));

    // 3. Import malformed JSON files
    let bad_json_path = dir.join("bad.json");
    std::fs::write(&bad_json_path, "{invalid json").unwrap();

    assert!(repo.import_profiles(&bad_json_path).is_err());
    assert!(repo.import_single_profile(&bad_json_path).is_err());
    assert!(repo.import_engines(&bad_json_path).is_err());
    assert!(repo.import_single_engine(&bad_json_path).is_err());
    assert!(repo.import_config(&bad_json_path).is_err());

    // 4. Import profiles from non-existent file
    let non_existent_path = dir.join("does_not_exist.json");
    assert!(repo.import_profiles(&non_existent_path).is_err());
    assert!(repo.import_single_profile(&non_existent_path).is_err());
    assert!(repo.import_engines(&non_existent_path).is_err());
    assert!(repo.import_single_engine(&non_existent_path).is_err());
    assert!(repo.import_config(&non_existent_path).is_err());

    cleanup("repo_errors");
}

#[test]
fn test_export_decrypted_journal_success() {
    let dir = temp_dir("export_journal_success");
    let enc_path = dir.join("journal.enc");
    let out_path = dir.join("journal_decrypted.json");

    let sample_data = r#"{"key": "value", "number": 42}"#;
    
    // Encrypt using DPAPI if possible
    if let Some(encrypted) = dpapi::encrypt(sample_data.as_bytes()) {
        std::fs::write(&enc_path, encrypted).unwrap();
        let result = export_decrypted_journal(&enc_path, &out_path);
        assert!(result.is_ok());
        assert!(out_path.exists());
        let read_back = std::fs::read_to_string(&out_path).unwrap();
        assert!(read_back.contains("42"));
        assert!(read_back.contains("value"));
    }
    
    cleanup("export_journal_success");
}

#[test]
fn test_action_logger_rotation() {
    let dir = temp_dir("logger_rotation");
    let log_path = dir.join("vibepilot.log");
    
    let logger = ActionLogger::new(log_path.clone());
    
    let chunk = "a".repeat(100 * 1024); // 100 KB
    for _ in 0..52 {
        logger.info(&chunk);
    }
    
    assert!(log_path.exists());
    let size_before = fs::metadata(&log_path).unwrap().len();
    assert!(size_before >= 5 * 1024 * 1024);
    
    logger.info("trigger rotation");
    
    let path_1 = log_path.with_extension("log.1");
    assert!(path_1.exists());
    assert!(log_path.exists());
    
    let size_new = fs::metadata(&log_path).unwrap().len();
    assert!(size_new < 1000); 
    
    for _ in 0..52 {
        logger.info(&chunk);
    }
    logger.info("trigger rotation 2");
    
    let path_2 = log_path.with_extension("log.2");
    assert!(path_2.exists());
    assert!(path_1.exists());
    
    for _ in 0..52 {
        logger.info(&chunk);
    }
    logger.info("trigger rotation 3");
    
    let path_3 = log_path.with_extension("log.3");
    assert!(path_3.exists());
    
    cleanup("logger_rotation");
}

