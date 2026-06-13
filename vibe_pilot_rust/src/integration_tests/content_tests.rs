#[test]
fn test_engine_presets_have_defaults() {
    use crate::config::EnginePresets;
    let presets = EnginePresets::default();
    assert!(!presets.lm_studio.url.is_empty());
    assert!(!presets.ollama.url.is_empty());
    assert!(!presets.custom.url.is_empty());
    assert!(!presets.lm_studio.modeles.is_empty());
    assert!(!presets.ollama.modeles.is_empty());
    assert!(!presets.custom.modeles.is_empty());
}

#[test]
fn test_engine_preset_names() {
    use crate::config::EnginePresets;
    let presets = EnginePresets::default();
    assert_eq!(presets.lm_studio.url, "http://127.0.0.1:1234/v1/chat/completions");
    assert_eq!(presets.ollama.url, "http://127.0.0.1:11434/v1/chat/completions");
    assert_eq!(presets.custom.url, "http://localhost:8000/v1/chat/completions");
}

#[test]
fn test_default_config_fields() {
    use crate::config::SavedConfig;
    let config = SavedConfig::default();
    assert!(config.activer_son);
    assert!(config.auto_validate);
    assert!(config.theme_sombre);
}
