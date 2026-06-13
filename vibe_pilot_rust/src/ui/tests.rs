#[cfg(test)]
mod ui_tests {
    use eframe::egui;
    use crate::app::{VibePilotApp, Tab, PendingAction, StructuredStep};
    use crate::common::{temp_dir, cleanup};
    use crate::ui::render_main_window;
    use crate::ui::components::*;
    use crate::ui::global_config::*;
    use crate::ui::setup::*;
    use crate::ui::prompt_editor::*;
    use crate::ui::console_timeline::*;


    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_ui_components_direct() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_components_direct");
        let mut app = VibePilotApp::new(&egui_ctx);

        // Fill timeline actions
        app.action_history = vec![
            ("THINK".to_string(), "Thinking".to_string()),
            ("WAIT".to_string(), "Waiting".to_string()),
            ("COOL".to_string(), "Cooldown".to_string()),
            ("SCROLL".to_string(), "Scrolling".to_string()),
            ("CLICK_AND_TYPE".to_string(), "Typing".to_string()),
            ("SUCCESS".to_string(), "Done".to_string()),
            ("FAIL".to_string(), "Failed".to_string()),
            ("SLEEP".to_string(), "Sleeping".to_string()),
            ("STATIC".to_string(), "Static screen".to_string()),
            ("ERROR".to_string(), "Err".to_string()),
            ("UNKNOWN".to_string(), "?".to_string()),
        ];
        app.logs = vec!["Log line 1".to_string(), "Log line 2".to_string()];

        app.structured_steps = vec![
            StructuredStep {
                action_type: "THINK".to_string(),
                tooltip: "Thinking".to_string(),
                logs: vec!["[12:00:00] Log 1".to_string(), "[12:00:01] Log 2".to_string()],
                report: "[12:00:00] Report step".to_string(),
            },
            StructuredStep {
                action_type: "CLICK_AND_TYPE".to_string(),
                tooltip: "Clicked".to_string(),
                logs: vec![],
                report: String::new(),
            },
            StructuredStep {
                action_type: "SUCCESS".to_string(),
                tooltip: "Done".to_string(),
                logs: vec!["[12:00:05] Success log".to_string()],
                report: String::new(),
            },
        ];

        // Populate undo history to render the Undo button
        let snapshot = crate::orchestrator::session_memory::ActionSnapshot {
            timestamp: "12:00:00".to_string(),
            action_type: "CLICK_AND_TYPE".to_string(),
            action_description: "Clicked".to_string(),
            relative_click_position: vec![0.5, 0.5],
            text_typed: Some("hello".to_string()),
            scroll_value: 0,
            screenshot_hash: 0,
            window_title: "Mock Window".to_string(),
            reflection_feedback: None,
            active_task_id: Some(42),
        };
        app.orchestrator.undo_history.lock().unwrap().push(snapshot);

        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Test rendering of individual components
                render_timeline(ui, &mut app);
                render_timeline_vertical(ui, &mut app, 300.0);
                
                // Test console in both tabs (English)
                app.current_config.langue = "English".to_string();
                app.bottom_tab = 0; // logs
                render_activity_console(ui, &mut app, 150.0);
                app.bottom_tab = 1; // report
                render_activity_console(ui, &mut app, 150.0);
                
                // Test console in both tabs (French)
                app.current_config.langue = "Français".to_string();
                app.bottom_tab = 0;
                render_activity_console(ui, &mut app, 150.0);
                app.bottom_tab = 1;
                render_activity_console(ui, &mut app, 150.0);

                // Split timeline and logs
                render_console_and_timeline_split(ui, &mut app, 200.0);

                // Status controls
                app.is_running = true;
                app.status_color = "orange".to_string();
                render_status_controls(ui, &mut app);
                app.is_running = false;
                render_status_controls(ui, &mut app);

                app.pause_mode = true;
                render_status_controls(ui, &mut app);
                app.pause_mode = false;
                render_status_controls(ui, &mut app);
            });
        });

        // Call copy helpers to cover clipboard logic in tests
        copy_logs_to_clipboard(&egui_ctx, &app.logs);
        copy_report_to_clipboard(&egui_ctx, &app.report_content);

        cleanup("ui_components_direct");
    }

    #[test]
    fn test_ui_global_config_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_global_config_rendering");
        let mut app = VibePilotApp::new(&egui_ctx);

        // Try different configurations to test conditional branches
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Configs (dedicated vision, task decompose, French, API key auth)
                app.current_config.langue = "Français".to_string();
                app.current_config.utiliser_moteur_vision_dedie = true;
                app.current_config.decomposer_taches = true;
                app.show_add_engine = true;
                app.new_engine_name = "MyVisionEngine".to_string();
                app.new_engine_url = "http://my-vision-engine:8000/v1".to_string();
                app.current_config.auth_mode = "api_key".to_string();
                app.current_config.auth_mode_vision = "api_key".to_string();
                
                // Add a custom preset
                app.engine_presets.insert("CustomEngine".to_string(), crate::config::EngineProfile {
                    url: "http://custom".to_string(),
                    modeles: vec!["custom-model".to_string()],
                });
                app.current_config.moteur = "CustomEngine".to_string();
                app.current_config.moteur_vision = "CustomEngine".to_string();

                render_config_tab(ui, &mut app);

                // Configs (no vision, no task decompose, English, Basic auth)
                app.current_config.langue = "English".to_string();
                app.current_config.utiliser_moteur_vision_dedie = false;
                app.current_config.decomposer_taches = false;
                app.show_add_engine = false;
                app.current_config.auth_mode = "basic_auth".to_string();
                render_config_tab(ui, &mut app);

                // Configs (dedicated vision, English, Basic auth for vision)
                app.current_config.utiliser_moteur_vision_dedie = true;
                app.current_config.auth_mode_vision = "basic_auth".to_string();
                render_config_tab(ui, &mut app);
            });
        });

        cleanup("ui_global_config_rendering");
    }

    #[test]
    fn test_ui_setup_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_setup_rendering");
        let mut app = VibePilotApp::new(&egui_ctx);

        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.current_config.langue = "English".to_string();
                app.storage_status_message = "Error: some error".to_string();
                app.current_config.garder_ecran_actif = true;
                app.current_config.theme_sombre = true;
                app.current_config.economie_ecriture_ssd = true;
                app.current_config.activer_compression_historique = true;
                app.current_config.activer_recadrage_workspace = true;
                app.current_config.verifier_placement_souris = true;
                app.current_config.detecter_activite_utilisateur = true;
                app.current_config.activer_reflexion = true;
                app.current_config.prompt_reprise = Some("Resume".to_string());
                render_setup_tab(ui, &mut app);

                app.current_config.langue = "Français".to_string();
                app.storage_status_message = "Success message".to_string();
                app.current_config.zoom_facteur = Some(1.25);
                app.current_config.garder_ecran_actif = false;
                app.current_config.theme_sombre = false;
                app.current_config.economie_ecriture_ssd = false;
                app.current_config.activer_compression_historique = false;
                app.current_config.activer_recadrage_workspace = false;
                app.current_config.verifier_placement_souris = false;
                app.current_config.detecter_activite_utilisateur = false;
                app.current_config.activer_reflexion = false;
                app.current_config.prompt_reprise = None;
                render_setup_tab(ui, &mut app);
            });
        });

        cleanup("ui_setup_rendering");
    }

    #[test]
    fn test_ui_confirmations_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_confirmations_rendering");
        let mut app = VibePilotApp::new(&egui_ctx);

        // Test all window popups
        app.show_delete_confirm = Some("profile_to_delete".to_string());
        app.show_rename_profile = Some("old_name".to_string());
        app.show_reset_confirm = true;
        app.show_profile_ready_popup = Some("new_profile".to_string());
        app.pending_action = Some(PendingAction {
            action: "CLICK".to_string(),
            text_to_type: "rm -rf /".to_string(), // Dangerous keyword
            scroll_value: 120,
        });

        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        // Test safe action confirmation
        app.pending_action = Some(PendingAction {
            action: "CLICK".to_string(),
            text_to_type: "echo hello".to_string(), // Safe
            scroll_value: 0,
        });
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        // Test save success popups
        app.show_save_success_popup = Some("config".to_string());
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        app.show_save_success_popup = Some("global_config".to_string());
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        app.show_save_success_popup = Some("engine:TestEngine".to_string());
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        app.show_save_success_popup = Some("test_profile".to_string());
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            render_main_window(ctx, &mut app);
        });

        cleanup("ui_confirmations_rendering");
    }

    #[test]
    fn test_is_user_active_now_compiles() {
        let _ = is_user_active_now();
    }

    #[test]
    fn test_play_beep_compiles() {
        // Just call to verify no panic
        play_system_beep();
    }

    #[test]
    fn test_ui_prompt_editor_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_prompt_editor_rendering");
        let mut app = VibePilotApp::new(&egui_ctx);

        // English language
        app.current_config.langue = "English".to_string();
        app.selected_profile = "test_profile".to_string();
        
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_prompt_editor_tab(ui, &mut app);
            });
        });

        // French language + make it modified to cover is_profile_modified branches
        app.current_config.langue = "Français".to_string();
        app.current_config.contexte = "modified".to_string();
        
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_prompt_editor_tab(ui, &mut app);
            });
        });

        cleanup("ui_prompt_editor_rendering");
    }

    #[test]
    fn test_ui_task_graph_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("ui_task_graph_rendering");
        let mut app = VibePilotApp::new(&egui_ctx);

        // 1. Test rendering with MOCK_TASK_GRAPH = None
        crate::orchestrator::tests::MOCK_TASK_GRAPH.with(|g| *g.borrow_mut() = None);
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui::task_graph::render_task_graph_tab(ui, &mut app);
            });
        });

        // 2. Test rendering with MOCK_TASK_GRAPH = Some and all task statuses + cycle dependency
        let mut graph = crate::memory::TaskGraph::new();
        let mut t1 = crate::memory::TaskNode::new(crate::memory::TaskId(1), "Task 1".to_string(), vec![crate::memory::TaskId(2)]);
        t1.status = crate::memory::TaskStatus::Pending;
        let mut t2 = crate::memory::TaskNode::new(crate::memory::TaskId(2), "Task 2".to_string(), vec![crate::memory::TaskId(1)]);
        t2.status = crate::memory::TaskStatus::InProgress;
        let mut t3 = crate::memory::TaskNode::new(crate::memory::TaskId(3), "Task 3".to_string(), vec![]);
        t3.status = crate::memory::TaskStatus::Completed;
        let mut t4 = crate::memory::TaskNode::new(crate::memory::TaskId(4), "Task 4".to_string(), vec![]);
        t4.status = crate::memory::TaskStatus::Failed { reason: "error".to_string() };
        let mut t5 = crate::memory::TaskNode::new(crate::memory::TaskId(5), "Task 5".to_string(), vec![]);
        t5.status = crate::memory::TaskStatus::Blocked;
        let mut t6 = crate::memory::TaskNode::new(crate::memory::TaskId(6), "Task 6".to_string(), vec![]);
        t6.status = crate::memory::TaskStatus::Skipped;

        graph.tasks.insert(t1.id, t1);
        graph.tasks.insert(t2.id, t2);
        graph.tasks.insert(t3.id, t3);
        graph.tasks.insert(t4.id, t4);
        graph.tasks.insert(t5.id, t5);
        graph.tasks.insert(t6.id, t6);

        app.config_repo.save_task_graph(Some(graph));

        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui::task_graph::render_task_graph_tab(ui, &mut app);
            });
        });

        cleanup("ui_task_graph_rendering");
    }

    #[test]
    fn test_setup_tab_module_exists() {
        // Verify the module compiles and is accessible
        use crate::ui::setup;
        // The render_setup_tab function is UI-only, but we verify the module loads
        let _ = setup::render_setup_tab;
    }

    #[test]
    fn test_saved_config_default_fields_used_by_setup_tab() {
        let config = crate::config::SavedConfig::default();
        // Fields that the setup tab reads and displays
        assert!(config.activer_son);
        assert!(config.activer_tooltips);
        assert!(config.auto_validate);
        assert!(!config.auto_validate_dangerous);
        assert!(config.theme_sombre);
        assert_eq!(config.langue, "English");
        assert!(!config.verifier_placement_souris);
        assert!(!config.detecter_activite_utilisateur);
        assert!(config.economie_ecriture_ssd);
        assert!(config.garder_ecran_actif);
        assert!(config.zoom_facteur.is_none());
        assert!(config.dossier_sauvegarde_captures.is_none());
    }

    #[test]
    fn test_saved_config_theme_sombre_toggle() {
        let mut config = crate::config::SavedConfig::default();
        config.theme_sombre = true;
        assert!(config.theme_sombre);
        config.theme_sombre = false;
        assert!(!config.theme_sombre);
    }

    #[test]
    fn test_saved_config_auto_validate_fields() {
        let mut config = crate::config::SavedConfig::default();
        config.auto_validate = false;
        config.auto_validate_dangerous = true;
        assert!(!config.auto_validate);
        assert!(config.auto_validate_dangerous);
    }

    #[test]
    fn test_saved_config_detect_activity_field() {
        let mut config = crate::config::SavedConfig::default();
        config.detecter_activite_utilisateur = true;
        assert!(config.detecter_activite_utilisateur);
    }

    #[test]
    fn test_saved_config_verify_cursor_field() {
        let mut config = crate::config::SavedConfig::default();
        config.verifier_placement_souris = true;
        assert!(config.verifier_placement_souris);
    }

    #[test]
    fn test_saved_config_zoom_factor_values() {
        let mut config = crate::config::SavedConfig::default();
        // Test various zoom factor values used in the setup tab's zoom combo
        config.zoom_facteur = Some(1.0);
        assert!((config.zoom_facteur.unwrap() - 1.0).abs() < 0.01);

        config.zoom_facteur = Some(1.25);
        assert!((config.zoom_facteur.unwrap() - 1.25).abs() < 0.01);

        config.zoom_facteur = Some(1.5);
        assert!((config.zoom_facteur.unwrap() - 1.5).abs() < 0.01);

        config.zoom_facteur = Some(1.75);
        assert!((config.zoom_facteur.unwrap() - 1.75).abs() < 0.01);

        config.zoom_facteur = Some(2.0);
        assert!((config.zoom_facteur.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_saved_config_language_values() {
        let mut config = crate::config::SavedConfig::default();
        config.langue = "Français".to_string();
        assert_eq!(config.langue, "Français");

        config.langue = "English".to_string();
        assert_eq!(config.langue, "English");
    }

    #[test]
    fn test_saved_config_prompt_reprise_field() {
        let mut config = crate::config::SavedConfig::default();
        assert!(config.prompt_reprise.is_none());

        config.prompt_reprise = Some("Resume from here".to_string());
        assert_eq!(config.prompt_reprise, Some("Resume from here".to_string()));

        // Clearing via None check (as done in setup tab)
        config.prompt_reprise = None;
        assert!(config.prompt_reprise.is_none());
    }

    #[test]
    fn test_setup_tab_config_clone_preserves_fields() {
        let mut config = crate::config::SavedConfig::default();
        config.activer_son = false;
        config.activer_tooltips = false;
        config.auto_validate = false;
        config.auto_validate_dangerous = true;
        config.verifier_placement_souris = true;
        config.detecter_activite_utilisateur = true;
        config.theme_sombre = false;
        config.langue = "Français".to_string();
        config.economie_ecriture_ssd = true;
        config.garder_ecran_actif = false;
        config.dossier_sauvegarde_captures = Some("/path/to/saves".to_string());

        let cloned = config.clone();
        assert!(!cloned.activer_son);
        assert!(!cloned.activer_tooltips);
        assert!(!cloned.auto_validate);
        assert!(cloned.auto_validate_dangerous);
        assert!(cloned.verifier_placement_souris);
        assert!(cloned.detecter_activite_utilisateur);
        assert!(!cloned.theme_sombre);
        assert_eq!(cloned.langue, "Français");
        assert!(cloned.economie_ecriture_ssd);
        assert!(!cloned.garder_ecran_actif);
        assert_eq!(cloned.dossier_sauvegarde_captures, Some("/path/to/saves".to_string()));
    }

    #[test]
    fn test_render_setup_tab_function_signature() {
        // Verify the function exists and is accessible
        let _ = crate::ui::setup::render_setup_tab;
    }
}

