use crate::app::VibePilotApp;
use crate::event_bus::CommandEvent;
use crate::common::{temp_dir, cleanup};
use eframe::egui;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_quick_start_profile_name_generation() {
    let _lock = TEST_LOCK.lock().unwrap();
    let name = "profile_qs_gen";
    let dir = temp_dir(name);

    // Point bootstrap config to temp dir
    let mut bootstrap = crate::config::load_bootstrap_config();
    let old_dir = bootstrap.storage_dir.clone();
    bootstrap.storage_dir = Some(dir.to_string_lossy().to_string());
    crate::config::save_bootstrap_config(&bootstrap);

    let ctx = egui::Context::default();
    let mut app = VibePilotApp::new(&ctx);

    // Initially both should point to "profile_01" because it's the first available profile
    assert_eq!(app.selected_profile, "profile_01");
    assert_eq!(app.quick_start_profile_name, "profile_01");

    // Set quick start profile name to custom value
    app.quick_start_profile_name = "profile_custom_qs".to_string();
    app.current_config.demande_generique = "a generic prompt request".to_string();

    // Run prompt generator simulation
    app.bus.emit_command(CommandEvent::CreateProfileWithPrompts {
        profile_name: "profile_custom_qs".to_string(),
        contexte: "c".to_string(),
        task: "t".to_string(),
        objectif: "o".to_string(),
        directives: "d".to_string(),
    });
    app.poll_events();

    // selected_profile should switch to "profile_custom_qs"
    assert_eq!(app.selected_profile, "profile_custom_qs");

    // The new profile config file should exist
    let list_profiles = app.config_repo.list_profiles();
    assert!(list_profiles.contains(&"profile_custom_qs".to_string()));

    // quick_start_profile_name should be updated to next available ("profile_01" is still empty/unused on disk)
    assert_eq!(app.quick_start_profile_name, "profile_01");

    // Clean up
    bootstrap.storage_dir = old_dir;
    crate::config::save_bootstrap_config(&bootstrap);
    cleanup(name);
}

#[test]
fn test_app_optimize_field_and_generate_prompts() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_optimize_generate");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Set a dummy URL that fails quickly
    app.current_config.url_api = "http://127.0.0.1:1/v1".to_string();
    app.current_config.demande_generique = "a generic request".to_string();
    
    // Test optimize field for all possible branches
    app.optimize_prompt_field("contexte");
    app.optimize_prompt_field("task");
    app.optimize_prompt_field("objectif");
    app.optimize_prompt_field("directives");
    app.optimize_prompt_field("user_feedback");
    app.optimize_prompt_field("invalid_field"); // should return early
    
    // Test generate prompts
    app.generate_prompts_from_request();
    
    // Test scans
    app.scan_models();
    app.scan_models_vision();
    
    // Yield to the tokio runtime to let the spawned tasks execute and fail
    app.rt.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    
    // We should see logs related to the attempts/failures
    app.poll_events();
    assert!(app.logs.iter().any(|l| l.contains("Sending Text LLM Request") || l.contains("Scanning models")));
    
    cleanup("app_optimize_generate");
}

#[test]
fn test_app_eframe_update() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_eframe_update");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Call eframe::App::update inside Context::run
    let mut dummy_bytes = [0u8; 1024];
    let frame = unsafe { &mut *(dummy_bytes.as_mut_ptr() as *mut eframe::Frame) };
    
    let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
        <VibePilotApp as eframe::App>::update(&mut app, ctx, frame);
    });
    
    cleanup("app_eframe_update");
}

#[test]
fn test_app_update_models_list_events() {
    let _lock = TEST_LOCK.lock().unwrap();
    let egui_ctx = egui::Context::default();
    let _dir = temp_dir("app_update_models");
    
    let mut app = VibePilotApp::new(&egui_ctx);
    
    // Test LM Studio suggestion update
    app.bus.emit_command(CommandEvent::UpdateModelsList {
        engine: "LM Studio".to_string(),
        models: vec!["lm-model-1".to_string()],
    });
    app.poll_events();
    assert_eq!(app.engine_presets.lm_studio.modeles, vec!["lm-model-1".to_string()]);
    
    // Test Ollama suggestion update
    app.bus.emit_command(CommandEvent::UpdateModelsList {
        engine: "Ollama".to_string(),
        models: vec!["ollama-model-1".to_string()],
    });
    app.poll_events();
    assert_eq!(app.engine_presets.ollama.modeles, vec!["ollama-model-1".to_string()]);

    // Test Perso / Autre suggestion update
    app.bus.emit_command(CommandEvent::UpdateModelsList {
        engine: "Perso / Autre".to_string(),
        models: vec!["custom-model-1".to_string()],
    });
    app.poll_events();
    assert_eq!(app.engine_presets.custom.modeles, vec!["custom-model-1".to_string()]);

    // Test custom engine suggestions
    app.engine_presets.custom_engines.insert(
        "MyCustomEngine".to_string(),
        crate::config::EngineProfile {
            url: "url".to_string(),
            modeles: vec![],
        }
    );
    app.bus.emit_command(CommandEvent::UpdateModelsList {
        engine: "MyCustomEngine".to_string(),
        models: vec!["my-custom-model-1".to_string()],
    });
    app.poll_events();
    assert_eq!(
        app.engine_presets.custom_engines.get("MyCustomEngine").unwrap().modeles,
        vec!["my-custom-model-1".to_string()]
    );
    
    cleanup("app_update_models");
}
