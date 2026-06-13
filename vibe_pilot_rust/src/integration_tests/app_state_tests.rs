use crate::app::{VibePilotApp, Tab};
use crate::common::{temp_dir, cleanup};
use crate::event_bus::{NotificationEvent, CommandEvent};
use eframe::egui;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_app_creation_and_defaults() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_state_creation");
    
    let app = VibePilotApp::new(&egui_ctx);
    assert_eq!(app.active_tab, Tab::GlobalConfig);
    assert!(!app.is_running);
    assert_eq!(app.logs.len(), 0);
    assert!(app.current_config.activer_son);
    assert!(app.current_config.auto_validate);
    cleanup("app_state_creation");
}

#[test]
fn test_app_load_profile_resets_session() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let dir = temp_dir("app_load_profile_resets");
    let _ = std::fs::create_dir_all(&dir);
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Setup mock profile on disk
    let config = crate::config::SavedConfig {
        contexte: "profile context".to_string(),
        ..crate::config::SavedConfig::default()
    };
    app.config_repo.save_profile("reset_test_profile", &config);

    // Put some data into the active session
    app.action_history.push(("CLICK".to_string(), "c".to_string()));
    app.logs.push("Log message".to_string());
    app.report_content = "some report".to_string();
    app.status_text = "Running".to_string();
    app.status_color = "green".to_string();
    app.is_running = true;

    // Load profile and verify it resets the session
    assert!(app.load_profile("reset_test_profile"));
    app.poll_events();
    assert_eq!(app.action_history.len(), 0);
    assert_eq!(app.logs.len(), 1); // Only the "Profile loaded. Session reset." log is present
    assert_eq!(app.report_content, "");
    assert_eq!(app.status_text, "Ready");
    assert_eq!(app.status_color, "grey");
    assert!(!app.is_running);
    assert_eq!(app.current_config.contexte, "profile context");

    cleanup("app_load_profile_resets");
}

#[test]
fn test_app_translation_helper() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_translation");
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // English translation check
    app.current_config.langue = "English".to_string();
    assert_eq!(app.t("btn_reinit"), "🔄 Reset All");
    
    // French translation check
    app.current_config.langue = "Français".to_string();
    assert_eq!(app.t("btn_reinit"), "🔄 Réinitialiser tout");
    
    cleanup("app_translation");
}

#[test]
fn test_app_reset_settings() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_reset_settings");
    
    // Point bootstrap config to _dir
    let mut bootstrap = crate::config::load_bootstrap_config();
    bootstrap.storage_dir = Some(_dir.to_string_lossy().to_string());
    crate::config::save_bootstrap_config(&bootstrap);

    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Modify fields
    app.current_config.contexte = "Non-default context".to_string();
    app.current_config.langue = "Français".to_string();
    app.current_config.zoom_facteur = Some(1.5);
    app.current_config.auto_validate = false;
    
    // Reset settings
    app.reset_all_settings();
    
    // Check defaults are restored
    assert_eq!(app.current_config.contexte, crate::config::DEFAULT_CONTEXT);
    assert_eq!(app.current_config.langue, "English");
    assert_eq!(app.current_config.zoom_facteur, None);
    assert!(app.current_config.auto_validate);
    
    cleanup("app_reset_settings");
}

#[test]
fn test_app_engine_management() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_engine_mgmt");
    
    // Point bootstrap config to _dir
    let mut bootstrap = crate::config::load_bootstrap_config();
    bootstrap.storage_dir = Some(_dir.to_string_lossy().to_string());
    crate::config::save_bootstrap_config(&bootstrap);

    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Add a preset
    let custom_profile = crate::config::EngineProfile {
        url: "http://my-custom-engine:9999/v1".to_string(),
        modeles: vec!["my-custom-model".to_string()],
    };
    app.engine_presets.insert("MyCustomEngine".to_string(), custom_profile.clone());
    app.config_repo.save_engines(&app.engine_presets);
    app.config_repo.save_now().unwrap();
    
    // Reload in new app to verify persistence
    let app_reloaded = VibePilotApp::new(&egui_ctx);
    let loaded_preset = app_reloaded.engine_presets.get("MyCustomEngine");
    assert!(loaded_preset.is_some());
    assert_eq!(loaded_preset.unwrap().url, "http://my-custom-engine:9999/v1");
    
    // Delete the preset
    let mut app_deleted = app_reloaded;
    app_deleted.engine_presets.remove("MyCustomEngine");
    app_deleted.config_repo.save_engines(&app_deleted.engine_presets);
    app_deleted.config_repo.save_now().unwrap();
    
    // Reload again to verify deletion
    let app_final = VibePilotApp::new(&egui_ctx);
    assert!(app_final.engine_presets.get("MyCustomEngine").is_none());
    
    cleanup("app_engine_mgmt");
}

#[test]
fn test_app_storage_migration() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let src_dir = temp_dir("app_migration_src");
    let dest_dir = temp_dir("app_migration_dest");
    
    // Point bootstrap config to src_dir
    let mut bootstrap = crate::config::load_bootstrap_config();
    bootstrap.storage_dir = Some(src_dir.to_string_lossy().to_string());
    crate::config::save_bootstrap_config(&bootstrap);

    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Save some custom config in current repo
    app.current_config.contexte = "Migration Test Contexte".to_string();
    app.config_repo.save_config(&app.current_config);
    app.config_repo.save_now().unwrap();
    
    // Migrate storage
    let dest_path_str = dest_dir.to_string_lossy().to_string();
    let result = app.migrate_storage(dest_path_str.clone());
    assert!(result.is_ok());
    
    // Verify bootstrap config updated
    let loaded_bootstrap = crate::config::load_bootstrap_config();
    assert_eq!(loaded_bootstrap.storage_dir, Some(dest_path_str.clone()));
    
    // Verify app's config_repo updated
    let new_save_path = app.config_repo.get_save_path();
    assert!(new_save_path.starts_with(&dest_dir));
    
    // Verify the config is loaded/copied correctly
    assert_eq!(app.current_config.contexte, "Migration Test Contexte");
    
    cleanup("app_migration_src");
    cleanup("app_migration_dest");
}

#[test]
fn test_app_ui_rendering() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_ui_rendering");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Render default tab (GlobalConfig)
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    // Switch to PromptEditor
    app.active_tab = Tab::PromptEditor;
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    // Switch to Console
    app.active_tab = Tab::Console;
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    // Switch to Setup
    app.active_tab = Tab::Setup;
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    // Setup pending dangerous action modal
    app.pending_action = Some(crate::app::PendingAction {
        action: "CLICK".to_string(),
        text_to_type: "rm -rf /".to_string(),
        scroll_value: 0,
    });
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    // Setup pending safe action modal
    app.pending_action = Some(crate::app::PendingAction {
        action: "CLICK".to_string(),
        text_to_type: "safe command".to_string(),
        scroll_value: 0,
    });
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        crate::ui::render_main_window(ctx, &mut app);
    });

    cleanup("app_ui_rendering");
}

#[test]
fn test_app_orchestrator_controls() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_orch_controls");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    assert!(!app.is_running);
    
    app.start_orchestrator();
    assert!(app.is_running);
    
    app.stop_orchestrator();
    assert!(!app.is_running);
    
    cleanup("app_orch_controls");
}

#[test]
fn test_app_event_processing() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_event_proc");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Test Event::Emit Log
    app.bus.emit_notification(NotificationEvent::Log("Test log message".to_string()));
    app.poll_events();
    assert!(app.logs.iter().any(|l| l.contains("Test log message")));
    
    // Test Event::Emit UpdateStatus
    app.bus.emit_notification(NotificationEvent::UpdateStatus { text: "Custom Status".to_string(), color: "blue".to_string() });
    app.poll_events();
    assert_eq!(app.status_text, "Custom Status");
    assert_eq!(app.status_color, "blue");
    
    // Test Event::Emit AppendAction
    app.bus.emit_notification(NotificationEvent::AppendAction { action: "CLICK".to_string(), display: "Clicked Button".to_string() });
    app.poll_events();
    assert_eq!(app.action_history.last().unwrap(), &("CLICK".to_string(), "Clicked Button".to_string()));
    
    // Test Event::Emit SelectTab
    app.bus.emit_command(CommandEvent::SelectTab(2)); // Console
    app.poll_events();
    assert_eq!(app.active_tab, Tab::Console);
    
    // Test Event::Emit UpdateField
    app.bus.emit_command(CommandEvent::UpdateField { field: "contexte".to_string(), value: "New Context Content".to_string() });
    app.poll_events();
    assert_eq!(app.current_config.contexte, "New Context Content");
    
    // Test Event::Emit UpdateAllFields
    app.bus.emit_command(CommandEvent::UpdateAllFields {
        contexte: "c".to_string(),
        task: "t".to_string(),
        objectif: "o".to_string(),
        directives: "d".to_string(),
    });
    app.poll_events();
    assert_eq!(app.current_config.contexte, "c");
    assert_eq!(app.current_config.task, "t");
    assert_eq!(app.current_config.objectif, "o");
    assert_eq!(app.current_config.directives, "d");
    
    // Test Event::Emit ShowActionConfirmation & ClearActionConfirmation
    app.bus.emit_command(CommandEvent::ShowActionConfirmation {
        action: "CLICK".to_string(),
        text: "Hello".to_string(),
        scroll: 42,
    });
    app.poll_events();
    let pending = app.pending_action.clone().unwrap();
    assert_eq!(pending.action, "CLICK");
    assert_eq!(pending.text_to_type, "Hello");
    assert_eq!(pending.scroll_value, 42);
    
    app.bus.emit_notification(NotificationEvent::ClearActionConfirmation);
    app.poll_events();
    assert!(app.pending_action.is_none());
    
    cleanup("app_event_proc");
}

#[test]
fn test_profile_name_generation_and_rename() {
    let _lock = TEST_LOCK.lock().unwrap();
    let name = "profile_gen_rename";
    let dir = temp_dir(name);
    
    // Point bootstrap config to temp dir
    let mut bootstrap = crate::config::load_bootstrap_config();
    let old_dir = bootstrap.storage_dir.clone();
    bootstrap.storage_dir = Some(dir.to_string_lossy().to_string());
    crate::config::save_bootstrap_config(&bootstrap);

    let ctx = egui::Context::default();
    let mut app = VibePilotApp::new(&ctx);

    // Verify initial available profile name
    let first_name = app.get_first_available_profile_name();
    assert_eq!(first_name, "profile_01");

    // Save a profile named profile_01
    app.selected_profile = "profile_01".to_string();
    app.config_repo.save_profile("profile_01", &app.current_config);

    // Now first available profile name should be profile_02
    let second_name = app.get_first_available_profile_name();
    assert_eq!(second_name, "profile_02");

    // Test renaming profile_01 to profile_03
    // 1. Verify profile_01 exists and profile_03 does not
    let list_before = app.config_repo.list_profiles();
    assert!(list_before.contains(&"profile_01".to_string()));
    assert!(!list_before.contains(&"profile_03".to_string()));

    // 2. Perform rename logic
    if let Some(cfg) = app.config_repo.load_profile("profile_01") {
        app.config_repo.save_profile("profile_03", &cfg);
        app.config_repo.delete_profile("profile_01");
        app.selected_profile = "profile_03".to_string();
    }

    // 3. Verify results
    let list_after = app.config_repo.list_profiles();
    assert!(!list_after.contains(&"profile_01".to_string()));
    assert!(list_after.contains(&"profile_03".to_string()));

    // Restore bootstrap config
    bootstrap.storage_dir = old_dir;
    crate::config::save_bootstrap_config(&bootstrap);
    cleanup(name);
}

