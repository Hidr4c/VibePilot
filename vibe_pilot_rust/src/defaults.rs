//! Dedicated factory for default configuration profiles.

use crate::config::{SavedConfig, ENGINE_LM_STUDIO, ENGINE_OLLAMA};

/// A factory to construct default settings and profiles for the application.
pub struct DefaultProfileFactory;

impl DefaultProfileFactory {
    /// Returns the default `SavedConfig` instance.
    pub fn build_default_config() -> SavedConfig {
        SavedConfig::default()
    }

    /// Returns the default `Default_Kilo_VSCode` profile configuration.
    pub fn build_kilo_vscode_profile() -> SavedConfig {
        SavedConfig {
            contexte: "Sprint 19 has been launched in Kilo Code extension inside VS Code. The sprint is partially completed. The AI must retrieve the remaining tasks from the Kilo sprint dashboard and continue executing them sequentially.".to_string(),
            objectif: "test coverage up to 80%.".to_string(),
            task: "You are the PC operator. Your target goal is to use Kilo extension in VS Code to bring test coverage up to 80%.".to_string(),
            directives: "- If you detect 'Considering next step...', 'Running...', or active test logs: the application is BUSY. Return 'WAIT'.\n- If the chat looks idle but the bottom is cut off, execute a 'SCROLL' action downwards (scroll_value: -6) to inspect the fold.\n- If and only if the application has completely stopped and provided the next prompt/input description (e.g., 'Prompt 18: ...'), extract it exactly and return 'CLICK_AND_TYPE'.".to_string(),
            demande_generique: "Avoir une couverture de test a 80% avec l'extension Kilo dans VS Code.".to_string(),
            fenetres_surveillees: vec!["Visual Studio Code".to_string()],
            moteur: ENGINE_LM_STUDIO.to_string(),
            url_api: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
            nom_modele: "qwen/qwen3.6-35b-a3b".to_string(),
            activer_son: true,
            activer_tooltips: true,
            langue: "English".to_string(),
            auto_validate: true,
            auto_validate_dangerous: false,
            prompt_reprise: Some("Fix the build errors: TS2322 in src/month-view.test.tsx and TS2307 (Cannot find module '@nidex/tool-multilanguage') in src/chat-bubble.tsx, src/chat-history.tsx, and src/chat-input.tsx.".to_string()),
            theme_sombre: true,
            zoom_facteur: None,
            ..SavedConfig::default()
        }
    }

    /// Returns the default `Template_Gravity_Pipeline` profile configuration.
    pub fn build_gravity_pipeline_profile() -> SavedConfig {
        SavedConfig {
            contexte: "Gravity Pipeline deployment system is active. The operator must verify build statuses and click deploy when successful.".to_string(),
            objectif: "Pipeline deployed successfully.".to_string(),
            task: "Monitor the Gravity build queue, wait for compile success, and trigger the deploy button.".to_string(),
            directives: "- Wait for green success indicator.\n- Click build details to audit log.\n- Authorize deploy by clicking the button.".to_string(),
            demande_generique: "Monitor and deploy build queue in Gravity pipeline.".to_string(),
            fenetres_surveillees: vec!["Gravity Pipeline".to_string()],
            moteur: ENGINE_OLLAMA.to_string(),
            url_api: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
            nom_modele: "llava:latest".to_string(),
            activer_son: true,
            activer_tooltips: true,
            langue: "English".to_string(),
            auto_validate: false,
            auto_validate_dangerous: false,
            prompt_reprise: None,
            theme_sombre: true,
            zoom_facteur: None,
            ..SavedConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_default_config() {
        let cfg = DefaultProfileFactory::build_default_config();
        assert_eq!(cfg.contexte, SavedConfig::default().contexte);
    }

    #[test]
    fn test_build_kilo_vscode_profile() {
        let cfg = DefaultProfileFactory::build_kilo_vscode_profile();
        assert!(cfg.contexte.contains("Kilo Code"));
    }

    #[test]
    fn test_build_gravity_pipeline_profile() {
        let cfg = DefaultProfileFactory::build_gravity_pipeline_profile();
        assert!(cfg.contexte.contains("Gravity Pipeline"));
    }
}
