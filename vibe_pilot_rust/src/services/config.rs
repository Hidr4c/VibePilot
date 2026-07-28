//! Configuration and profile management service.

use crate::config::{ConfigurationRepository, SavedConfig, EnginePresets, DEFAULT_CONTEXT, DEFAULT_OBJECTIF, DEFAULT_TASK, DEFAULT_DIRECTIVES, ALL_SCREENS_KEY};
use std::sync::Arc;

pub struct ConfigService {
    config_repo: Arc<dyn ConfigurationRepository>,
    engine_presets: EnginePresets,
}

impl ConfigService {
    pub fn new(config_repo: Arc<dyn ConfigurationRepository>) -> Self {
        let engine_presets = config_repo.load_engines();
        Self { config_repo, engine_presets }
    }

    pub fn config_repo(&self) -> &Arc<dyn ConfigurationRepository> {
        &self.config_repo
    }

    pub fn load_config(&self) -> SavedConfig {
        self.config_repo.load_config()
    }

    pub fn save_config(&self, config: &SavedConfig) {
        self.config_repo.save_config(config);
    }

    pub fn load_engines(&self) -> EnginePresets {
        self.engine_presets.clone()
    }

    pub fn save_engines(&mut self, presets: &EnginePresets) {
        self.config_repo.save_engines(presets);
        self.engine_presets = presets.clone();
    }

    pub fn load_profile(&mut self, name: &str) -> Option<SavedConfig> {
        let mut cfg = self.config_repo.load_profile(name)?;
        cfg.dernier_profil = name.to_string();
        Some(cfg)
    }

    pub fn save_profile(&mut self, name: &str, config: &SavedConfig) {
        self.config_repo.save_profile(name, config);
    }

    pub fn delete_profile(&mut self, name: &str) -> bool {
        self.config_repo.delete_profile(name)
    }

    pub fn list_profiles(&self) -> Vec<String> {
        self.config_repo.list_profiles()
    }

    pub fn save_task_graph(&self, graph: Option<crate::memory::TaskGraph>) {
        self.config_repo.save_task_graph(graph);
    }

    pub fn load_task_graph(&self) -> Option<crate::memory::TaskGraph> {
        self.config_repo.load_task_graph()
    }

    pub fn get_first_available_profile_name(&self) -> String {
        use std::collections::HashSet;
        let existing_profiles: HashSet<String> = self.list_profiles().into_iter().collect();
        let mut i = 1;
        loop {
            let name = format!("profile_{:02}", i);
            if !existing_profiles.contains(&name) {
                return name;
            }
            i += 1;
        }
    }

    pub fn reset_all_settings(&mut self) {
        let config = self.config_repo.load_config();
        let mut reset = config.clone();
        reset.contexte = DEFAULT_CONTEXT.to_string();
        reset.objectif = DEFAULT_OBJECTIF.to_string();
        reset.task = DEFAULT_TASK.to_string();
        reset.directives = DEFAULT_DIRECTIVES.to_string();
        reset.fenetres_surveillees = vec![ALL_SCREENS_KEY.to_string()];
        reset.langue = "English".to_string();
        reset.activer_son = true;
        reset.activer_tooltips = true;
        reset.auto_validate = true;
        reset.auto_validate_dangerous = false;
        reset.theme_sombre = true;
        reset.prompt_reprise = None;
        reset.zoom_facteur = None;
        reset.trace_actions_visuelles = false;
        self.config_repo.save_config(&reset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigRepository;
    use crate::common::{temp_dir, cleanup};

    #[test]
    fn test_config_service() {
        let path = temp_dir("service_config");
        let repo: Arc<dyn ConfigurationRepository> = Arc::new(ConfigRepository::new(path.clone()));
        let mut service = ConfigService::new(repo.clone());

        // Test config_repo accessor
        assert!(Arc::ptr_eq(service.config_repo(), &repo));

        // Test load/save config
        let mut cfg = service.load_config();
        cfg.nom_modele = "test-model-abc".to_string();
        service.save_config(&cfg);
        let loaded = service.load_config();
        assert_eq!(loaded.nom_modele, "test-model-abc");

        // Test load/save engines
        let mut engines = service.load_engines();
        engines.custom_engines.insert("test-engine".to_string(), crate::config::models::EngineProfile {
            url: "http://test".to_string(),
            modeles: vec!["model-a".to_string()],
        });
        service.save_engines(&engines);
        let loaded_engines = service.load_engines();
        assert!(loaded_engines.custom_engines.contains_key("test-engine"));

        // Test list/save/delete profiles
        let mut cfg_p = service.load_config();
        cfg_p.contexte = "profile context".to_string();
        service.save_profile("prof1", &cfg_p);
        let list = service.list_profiles();
        assert!(list.contains(&"prof1".to_string()));

        let loaded_p = service.load_profile("prof1").unwrap();
        assert_eq!(loaded_p.contexte, "profile context");
        assert_eq!(loaded_p.dernier_profil, "prof1");

        let deleted = service.delete_profile("prof1");
        assert!(deleted);
        let list_after = service.list_profiles();
        assert!(!list_after.contains(&"prof1".to_string()));

        // Test task graph
        service.save_task_graph(None);
        assert!(service.load_task_graph().is_none());

        // Test get_first_available_profile_name
        let name = service.get_first_available_profile_name();
        assert_eq!(name, "profile_01");

        // Test reset_all_settings
        service.reset_all_settings();
        let reset_cfg = service.load_config();
        assert_eq!(reset_cfg.langue, "English");

        cleanup("service_config");
    }
}
