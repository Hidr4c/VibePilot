use tokio::time::{sleep, Duration};
use crate::event_bus::{NotificationEvent};
use crate::orchestrator::{VibePilotOrchestrator, OrchestratorState};
use crate::orchestrator::session_memory::{ActionStep, SessionMemory, MAX_HISTORY_STEPS};
use crate::orchestrator::loop_breaker::{LoopBreaker, LoopAction};
use crate::orchestrator::helpers::{is_local_url, get_gpu_utilization};
use crate::memory::{TaskGraph, TaskId};

impl VibePilotOrchestrator {
    /// The main execution loop with safety limits.
    ///
    /// This loop runs until stopped by the user, until the LLM returns
    /// a `SUCCESS` action, or until safety limits are reached:
    /// - Maximum iterations: 1000
    /// - Maximum session duration: 1 hour
    pub async fn run_loop(&self) -> Result<(), String> {
        self.replay_manager.clear_full_session();
        *self.state.lock().unwrap() = OrchestratorState::Running;
        struct StateGuard<'a> {
            state: &'a std::sync::Arc<std::sync::Mutex<OrchestratorState>>,
        }
        impl<'a> Drop for StateGuard<'a> {
            fn drop(&mut self) {
                let mut s = self.state.lock().unwrap();
                if matches!(*s, OrchestratorState::Running | OrchestratorState::Paused | OrchestratorState::AwaitingApproval | OrchestratorState::Reanchoring) {
                    *s = OrchestratorState::Idle;
                }
            }
        }
        let _guard = StateGuard { state: &self.state };

        let result = self.run_loop_internal().await;
        if let Err(ref e) = result {
            *self.state.lock().unwrap() = OrchestratorState::Error(e.clone());
            let _ = self.replay_manager.save_replay(e);
        }
        result
    }

    async fn run_loop_internal(&self) -> Result<(), String> {
        self.bus.emit_notification(NotificationEvent::Log("Starting orchestration loop...".to_string()));

        let mut iteration_count = 0u64;
        let session_start = tokio::time::Instant::now();
        let mut loop_breaker = LoopBreaker::new();
        let mut escalate_to_desktop = false;

        let mut last_click_coordinates: Option<(f64, f64)> = None;
        let mut session_memory = SessionMemory::new();
        let mut window_recovery_attempted = false;
        let mut last_captured_image: Option<image::DynamicImage> = None;

        let mut coordinate_calibration_retries = 0;
        let mut base_click_coordinates: Option<(f64, f64)> = None;
        let mut last_active_task_id: Option<TaskId> = None;

        let mut task_graph: Option<TaskGraph>;
        let mut last_journal_snapshot = tokio::time::Instant::now();
        let mut last_objectif = String::new();
        let mut reflection_feedback: Option<String> = None;

        loop {
            // Safety check: iteration limit
            iteration_count += 1;
            self.check_safety_limits(iteration_count, session_start)?;

            // Hot-reload configuration dynamically
            let config = self.config_repo.load_config();
            if config.activer_compression_historique {
                session_memory.max_steps = 10;
            } else {
                session_memory.max_steps = MAX_HISTORY_STEPS;
            }

            // TaskGraph planning/initialization if active
            if config.decomposer_taches {
                task_graph = self.config_repo.load_task_graph();
                let obj_changed = last_objectif != config.objectif;
                if task_graph.is_none() || obj_changed {
                    last_objectif = config.objectif.clone();
                    task_graph = self.initialize_task_graph(&config).await;
                    self.config_repo.save_task_graph(task_graph.clone());
                }
            } else {
                if self.config_repo.load_task_graph().is_some() {
                    self.config_repo.save_task_graph(None);
                }
                task_graph = None; // Reset if disabled
            }

            // 0. Check if orchestrator has been stopped
            if !self.is_running() {
                self.bus.emit_notification(NotificationEvent::Log("Orchestration loop stopped.".to_string()));
                break;
            }

            // TaskGraph sub-task selection and termination checks
            let mut active_task_id: Option<TaskId> = None;
            if let Some(ref mut graph) = task_graph {
                if !self.select_active_task(graph, &mut active_task_id) {
                    self.config_repo.save_task_graph(task_graph.clone());
                    break;
                }
                self.config_repo.save_task_graph(task_graph.clone());
            }

            if active_task_id != last_active_task_id {
                last_active_task_id = active_task_id;
                coordinate_calibration_retries = 0;
                base_click_coordinates = None;
                last_click_coordinates = None;
            }

            // 1. Check for forced pause
            if self.is_paused() {
                *self.state.lock().unwrap() = OrchestratorState::Paused;
                self.bus.emit_notification(NotificationEvent::UpdateStatus {
                    text: "PAUSED".to_string(),
                    color: "red".to_string(),
                });
                self.cancelable_sleep(Duration::from_secs(1)).await;
                continue;
            } else {
                let mut s = self.state.lock().unwrap();
                if *s == OrchestratorState::Paused {
                    *s = OrchestratorState::Running;
                }
            }

            // 2. Verify target window visibility (skip if we are escalating to desktop capture)
            if !escalate_to_desktop {
                self.capturer.refresh_windows();
                if !self.capturer.is_target_visible(&config.fenetres_surveillees) {
                    self.bus.emit_notification(NotificationEvent::UpdateStatus {
                        text: "Waiting for target window...".to_string(),
                        color: "orange".to_string(),
                    });
                    self.cancelable_sleep(Duration::from_secs(2)).await;
                    continue;
                }
            }

            // 2.1. Visual Re-Anchoring Protocol: ensure target window is focused (skip if we are escalating to desktop capture)
            *self.state.lock().unwrap() = OrchestratorState::Reanchoring;
            let reanchored = self.run_window_reanchoring(&config, &mut escalate_to_desktop, &mut window_recovery_attempted).await;
            *self.state.lock().unwrap() = OrchestratorState::Running;
            if reanchored {
                continue;
            }

            // 2.5. Check GPU Busy Status for Local URLs
            if !cfg!(test) && is_local_url(&config.url_api) {
                if let Some(gpu_usage) = get_gpu_utilization().await {
                    if gpu_usage >= 90 {
                        self.bus.emit_notification(NotificationEvent::UpdateStatus {
                            text: format!("GPU busy ({}%)...", gpu_usage),
                            color: "blue".to_string(),
                        });
                        self.cancelable_sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }
            }

            // 2.6. Apply LoopBreaker escape strategies BEFORE capture
            if loop_breaker.escalation_level >= 3 {
                if loop_breaker.escalation_level >= 5 {
                    crate::orchestrator::helpers::navigate_loop_breaker_actions(&mut loop_breaker, self).await;
                    return Err("LOOP_DETECTED".to_string());
                }
                crate::orchestrator::helpers::navigate_loop_breaker_actions(&mut loop_breaker, self).await;
            }

            // 3. Capture and Decide
            self.bus.emit_notification(NotificationEvent::UpdateStatus {
                text: "Analyzing...".to_string(),
                color: "blue".to_string(),
            });
            self.bus.emit_notification(NotificationEvent::AppendAction {
                action: "THINK".to_string(),
                display: format!("Sending request to {} (URL: {})...", config.nom_modele, config.url_api),
            });

            let offsets = self.get_active_offsets(&config, escalate_to_desktop);
            if let Ok(mut lock) = self.active_workspace_offsets.lock() {
                *lock = offsets.clone();
            }
            if let Some(target) = offsets.first() {
                if config.activer_recadrage_workspace && !escalate_to_desktop {
                    if !config.fenetres_surveillees.is_empty() {
                        let expected = &config.fenetres_surveillees[0];
                        if target.titre == "Full Desktop" && expected != "Full Desktop" && !expected.is_empty() {
                            let msg = if config.langue == "Français" {
                                format!("⚠️ Attention : La fenêtre ciblée '{}' est introuvable ou minimisée. Repli sur 'Full Desktop' !", expected)
                            } else {
                                format!("⚠️ Warning: Monitored window '{}' could not be found or is minimized. Falling back to 'Full Desktop'!", expected)
                            };
                            self.bus.emit_notification(NotificationEvent::Log(msg));
                        } else {
                            self.bus.emit_notification(NotificationEvent::Log(format!(
                                "⚙️ Active Workspace: resolving capture area to '{}' [x={}, y={}, w={}, h={}]",
                                target.titre, target.left, target.top, target.width, target.height
                            )));
                        }
                    } else {
                        self.bus.emit_notification(NotificationEvent::Log(format!(
                            "⚙️ Active Workspace: resolving capture area to '{}' [x={}, y={}, w={}, h={}]",
                            target.titre, target.left, target.top, target.width, target.height
                        )));
                    }
                } else if !config.activer_recadrage_workspace && !config.fenetres_surveillees.is_empty() {
                    let expected = &config.fenetres_surveillees[0];
                    if expected != "Full Desktop" && !expected.is_empty() {
                        let msg = if config.langue == "Français" {
                            format!("⚠️ Attention : La fenêtre ciblée '{}' est sélectionnée, mais l'option « Recadrage Workspace » est désactivée dans vos paramètres ! Capture de tout l'écran par défaut.", expected)
                        } else {
                            format!("⚠️ Warning: Monitored window '{}' is selected, but 'Workspace Cropping' is disabled in your settings! Capturing Full Desktop instead.", expected)
                        };
                        self.bus.emit_notification(NotificationEvent::Log(msg));
                    }
                }
            }

            let screenshot = if config.activer_roi {
                self.capturer.capture_bbox(config.roi_x, config.roi_y, config.roi_width, config.roi_height)
            } else if offsets.len() == 1 {
                let target = &offsets[0];
                self.capturer.capture_bbox(target.left, target.top, target.width, target.height)
            } else {
                self.capturer.capture_desktop()
            }.or_else(|| self.capturer.capture_desktop());

            if let Some(mut img) = screenshot {
                self.pii_masker.mask_pii(&mut img);
                self.replay_manager.record_frame(&img);

                if !config.economie_ecriture_ssd {
                    let path = if let Some(ref dir) = config.dossier_sauvegarde_captures {
                        if !dir.trim().is_empty() {
                            let _ = std::fs::create_dir_all(dir);
                            std::path::Path::new(dir).join("last_capture.png")
                        } else {
                            std::path::PathBuf::from("last_capture.png")
                        }
                    } else {
                        std::path::PathBuf::from("last_capture.png")
                    };
                    let _ = img.save(path);
                }
                
                let use_zoom = config.pipeline_vision_avance || loop_breaker.repetition_count() >= 2;
                let mut zoomed_region: Option<crate::vision::ZoomedRegion> = None;
                let mut final_image = img.clone();

                if use_zoom {
                    self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
                        "🔍 Zoom Dynamique: Identification de la zone cible..."
                    } else {
                        "🔍 Dynamic Zoom: Identifying target region..."
                    }.to_string()));

                    match self.zoom_strategy.get_zoomed_region(
                        self.llm_client.as_ref(),
                        &img,
                        &config,
                    ).await {
                        Some(zoomed) => {
                            self.bus.emit_notification(NotificationEvent::Log(format!(
                                "🔍 Zoom Dynamique: Zone identifiée [x={}, y={}, w={}, h={}]",
                                zoomed.original_rect.0, zoomed.original_rect.1, zoomed.original_rect.2, zoomed.original_rect.3
                            )));
                            final_image = zoomed.zoomed_image.clone();
                            zoomed_region = Some(zoomed);
                        }
                        None => {
                            self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
                                "⚠️ Zoom Dynamique: Échec de l'identification de la ROI. Repli vers l'écran complet."
                            } else {
                                "⚠️ Dynamic Zoom: Failed to identify target region. Fallback to full screen."
                            }.to_string()));
                        }
                    }
                }

                // Draw past clicks
                self.draw_past_clicks(&mut final_image, &zoomed_region, &session_memory, last_click_coordinates);

                if !config.economie_ecriture_ssd {
                    let path = if let Some(ref dir) = config.dossier_sauvegarde_captures {
                        if !dir.trim().is_empty() {
                            let _ = std::fs::create_dir_all(dir);
                            std::path::Path::new(dir).join("last_capture_final.png")
                        } else {
                            std::path::PathBuf::from("last_capture_final.png")
                        }
                    } else {
                        std::path::PathBuf::from("last_capture_final.png")
                    };
                    let _ = final_image.save(&path);
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "📸 debug: Saved captures to folder: {}", path.display()
                    )));
                }

                self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
                    "🔍 Analyse du nouvel état de l'écran pour évaluer l'action précédente..."
                } else {
                    "🔍 Analyzing new screen state to verify previous action..."
                }.to_string()));

                let is_specific_window = !escalate_to_desktop
                    && !config.fenetres_surveillees.is_empty()
                    && !self.controller.is_desktop_title(&config.fenetres_surveillees[0]);

                if self.bus.emit_query(crate::event_bus::QueryEvent::GetOrchestratorRunning) == "false" {
                    break;
                }
                let response = self.get_llm_decision(
                    &config,
                    &img,
                    &final_image,
                    &zoomed_region,
                    &session_memory,
                    &task_graph,
                    active_task_id,
                    &reflection_feedback,
                    &loop_breaker,
                    last_click_coordinates,
                    escalate_to_desktop,
                    &offsets,
                    &last_captured_image,
                    is_specific_window,
                ).await;

                if let (Some(ref before), Some(last_click)) = (&last_captured_image, last_click_coordinates) {
                    let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(last_click.0, last_click.1, &offsets);
                    if let Some((dx, dy)) = self.auto_calibrator.calculate_drift(before, &img, (abs_x as u32, abs_y as u32)) {
                        self.bus.emit_notification(NotificationEvent::Log(format!(
                            "🎯 Auto-Calibration Loop: Detected drift dx={}, dy={}. Adjusting active screen offsets!",
                            dx, dy
                        )));
                        if let Ok(mut lock) = self.active_workspace_offsets.lock() {
                            for offset in lock.iter_mut() {
                                offset.left += dx;
                                offset.top += dy;
                            }
                        }
                    }
                }

                last_captured_image = Some(img.clone());

                if self.bus.emit_query(crate::event_bus::QueryEvent::GetOrchestratorRunning) == "false" {
                    break;
                }
                match response {
                    Ok(res) => {
                        match self.process_llm_decision(
                            &config,
                            res,
                            &img,
                            &zoomed_region,
                            &offsets,
                            &mut last_click_coordinates,
                            &mut session_memory,
                            &mut loop_breaker,
                            &mut escalate_to_desktop,
                            &mut task_graph,
                            active_task_id,
                            &mut reflection_feedback,
                            &mut coordinate_calibration_retries,
                            &mut base_click_coordinates,
                        ).await? {
                            Some(true) => {
                                self.config_repo.save_task_graph(task_graph.clone());
                                continue;
                            }
                            Some(false) => {
                                self.config_repo.save_task_graph(task_graph.clone());
                                break;
                            }
                            None => {}
                        }
                    }
                    Err(e) => {
                        self.bus.emit_notification(NotificationEvent::Log(format!("LLM error: {}", e)));
                        self.bus.emit_notification(NotificationEvent::AppendAction {
                            action: "ERROR".to_string(),
                            display: format!("LLM error: {}", e),
                        });
                        self.cancelable_sleep(Duration::from_secs(5)).await;
                    }
                }
            } else {
                self.bus.emit_notification(NotificationEvent::Log("Failed to capture screen".to_string()));
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "ERROR".to_string(),
                    display: "Failed to capture screen".to_string(),
                });
                self.cancelable_sleep(Duration::from_secs(2)).await;
            }

            // Save task graph to config repository (if active)
            self.config_repo.save_task_graph(task_graph.clone());

            // Periodically log snapshots to the journal
            if last_journal_snapshot.elapsed() >= std::time::Duration::from_secs(600) {
                last_journal_snapshot = tokio::time::Instant::now();
                if let Some(ref journal) = self.journal {
                    let snapshot = crate::config::SessionSnapshot {
                        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        objective: config.objectif.clone(),
                    };
                    let entry = crate::append_store::JournalEntry {
                        timestamp: chrono::Utc::now(),
                        entry_type: crate::append_store::JournalEntryType::SessionSnapshot(snapshot),
                    };
                    let _ = journal.append(&entry);

                    if let Some(ref tg) = task_graph {
                        let entry_tg = crate::append_store::JournalEntry {
                            timestamp: chrono::Utc::now(),
                            entry_type: crate::append_store::JournalEntryType::TaskGraphState(tg.clone()),
                        };
                        let _ = journal.append(&entry_tg);
                    }
                }
            }

            self.cancelable_sleep(Duration::from_millis(500)).await;
        }

        // Save final task graph state on loop exit
        self.config_repo.save_task_graph(task_graph.clone());

        self.bus.emit_notification(NotificationEvent::Log(format!(
            "Loop ended after {} iterations in {:.0}s",
            iteration_count,
            session_start.elapsed().as_secs()
        )));

        Ok(())
    }
}
