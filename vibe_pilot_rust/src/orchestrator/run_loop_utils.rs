use tokio::time::{sleep, Duration};
use crate::orchestrator::VibePilotOrchestrator;
use crate::orchestrator::session_memory::{SessionMemory, ActionStep, MAX_HISTORY_STEPS};
use crate::orchestrator::loop_breaker::{LoopBreaker, LoopAction};
use crate::orchestrator::helpers::draw_click_marker;
use crate::config::SavedConfig;
use crate::event_bus::NotificationEvent;
use crate::peripheral_controller::ScreenOffset;
use crate::memory::{TaskGraph, TaskId};
use crate::llm_client::LlmResponse;

impl VibePilotOrchestrator {
    /// Draws visual representation of past click coordinates sequentially on the capture.
    pub fn draw_past_clicks(
        &self,
        final_image: &mut image::DynamicImage,
        zoomed_region: &Option<crate::vision::ZoomedRegion>,
        session_memory: &SessionMemory,
        last_click_coordinates: Option<(f64, f64)>,
    ) {
        let mut past_clicks = Vec::new();
        for step in &session_memory.steps {
            if let Some(coords) = step.coordinates {
                past_clicks.push(coords);
            }
        }
        if let Some(coords) = last_click_coordinates {
            if past_clicks.last() != Some(&coords) {
                past_clicks.push(coords);
            }
        }

        let num_clicks = past_clicks.len();
        if num_clicks == 0 {
            return;
        }

        let click_colors: Vec<image::Rgba<u8>> = past_clicks
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if num_clicks <= 1 {
                    image::Rgba([255, 0, 0, 255]) // Bright Red
                } else {
                    let ratio = index as f32 / (num_clicks - 1) as f32;
                    let r = (80.0 + (255.0 - 80.0) * ratio) as u8;
                    let g = 0u8;
                    let b = (120.0 * (1.0 - ratio)) as u8;
                    image::Rgba([r, g, b, 255])
                }
            })
            .collect();

        struct ClickCluster {
            coords: (f64, f64),
            colors: Vec<image::Rgba<u8>>,
        }

        let mut clusters: Vec<ClickCluster> = Vec::new();
        for (index, &coords) in past_clicks.iter().enumerate() {
            let mut found = false;
            for cluster in &mut clusters {
                let dx = coords.0 - cluster.coords.0;
                let dy = coords.1 - cluster.coords.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 0.005 {
                    cluster.colors.push(click_colors[index]);
                    found = true;
                    break;
                }
            }
            if !found {
                clusters.push(ClickCluster {
                    coords,
                    colors: vec![click_colors[index]],
                });
            }
        }

        for cluster in &clusters {
            let (cx, cy) = cluster.coords;
            if let Some(ref zoomed) = zoomed_region {
                let ctx = &zoomed.crop_context;
                if cx >= ctx.crop_x && cx <= ctx.crop_x + ctx.crop_width && cy >= ctx.crop_y && cy <= ctx.crop_y + ctx.crop_height {
                    let zoom_cx = (cx - ctx.crop_x) / ctx.crop_width;
                    let zoom_cy = (cy - ctx.crop_y) / ctx.crop_height;
                    draw_click_marker(final_image, zoom_cx, zoom_cy, &cluster.colors);
                }
            } else {
                draw_click_marker(final_image, cx, cy, &cluster.colors);
            }
        }
    }

    /// Applies coordinate calibration based on repetition counts.
    pub fn apply_coordinate_calibration(
        &self,
        config: &SavedConfig,
        rx: &mut f64,
        ry: &mut f64,
        base_click_coordinates: &mut Option<(f64, f64)>,
        coordinate_calibration_retries: &mut u32,
    ) {
        let is_repetition_coords = if let Some((bx, by)) = *base_click_coordinates {
            let dx = *rx - bx;
            let dy = *ry - by;
            let dist = (dx * dx + dy * dy).sqrt();
            dist < 0.005
        } else {
            false
        };

        if is_repetition_coords {
            *coordinate_calibration_retries += 1;
            if let Some((bx, by)) = *base_click_coordinates {
                let (offset_x, offset_y) = match *coordinate_calibration_retries {
                    1 => (-0.015, 0.0), // Shift X left
                    2 => (0.015, 0.0),  // Shift X right
                    3 => (0.0, -0.015), // Shift Y up
                    4 => (0.0, 0.015),  // Shift Y down
                    5 => (-0.030, 0.0), // Shift X left further
                    6 => (0.030, 0.0),  // Shift X right further
                    7 => (0.0, -0.030), // Shift Y up further
                    8 => (0.0, 0.030),  // Shift Y down further
                    _ => (0.0, 0.0),
                };
                *rx = (bx + offset_x).clamp(0.0, 1.0);
                *ry = (by + offset_y).clamp(0.0, 1.0);

                let log_msg = if config.langue == "Français" {
                    format!("⚙️ Calibration Programmatique : L'IA a renvoyé les mêmes coordonnées. Correction par décalage (tentative {}) : déplacement vers x={:.3}, y={:.3}", *coordinate_calibration_retries, *rx, *ry)
                } else {
                    format!("⚙️ Programmatic Calibration: LLM returned same coordinates. Applying step correction (attempt {}): shifting to x={:.3}, y={:.3}", *coordinate_calibration_retries, *rx, *ry)
                };
                self.bus.emit_notification(NotificationEvent::Log(log_msg));
            }
        } else {
            *coordinate_calibration_retries = 0;
            *base_click_coordinates = Some((*rx, *ry));
        }
    }

    /// Compresses session history when steps limit is reached.
    pub async fn compress_session_history(
        &self,
        config: &SavedConfig,
        session_memory: &mut SessionMemory,
    ) {
        self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
            "📦 Compression de l'historique de session..."
        } else {
            "📦 Compressing session history..."
        }.to_string()));

        let mut old_steps_text = String::new();
        for i in 0..5 {
            if let Some(step) = session_memory.steps.get(i) {
                let coords = step.coordinates.map(|(x, y)| format!("({:.3}, {:.3})", x, y)).unwrap_or_default();
                old_steps_text.push_str(&format!("[{}] {} {}\n", step.timestamp, step.action_type, coords));
            }
        }

        let (target_url, target_model, auth_mode, auth_key, auth_login, auth_pass, timeout) = if config.utiliser_moteur_vision_dedie {
            (
                &config.url_api_vision,
                &config.nom_modele_vision,
                &config.auth_mode_vision,
                &config.auth_api_key_vision,
                &config.auth_login_vision,
                &config.auth_password_vision,
                config.request_timeout_secs_vision,
            )
        } else {
            (
                &config.url_api,
                &config.nom_modele,
                &config.auth_mode,
                &config.auth_api_key,
                &config.auth_login,
                &config.auth_password,
                config.request_timeout_secs,
            )
        };

        match self.llm_client.compress_history(
            &old_steps_text,
            &session_memory.compressed_history,
            &config.langue,
            target_url,
            target_model,
            auth_mode,
            auth_key,
            auth_login,
            auth_pass,
            timeout,
        ).await {
            Ok(new_summary) => {
                session_memory.compressed_history = new_summary;
                self.bus.emit_notification(NotificationEvent::Log(format!("✅ History compressed: {}", session_memory.compressed_history)));
                for _ in 0..5 {
                    session_memory.steps.pop_front();
                }
            }
            Err(err) => {
                self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Failed to compress history: {}", err)));
                while session_memory.steps.len() > crate::orchestrator::session_memory::MAX_HISTORY_STEPS {
                    session_memory.steps.pop_front();
                }
            }
        }
    }

    /// Processes the LLM decision (res), updating session memory, checking loop states,
    /// executing the action via `handle_decision`, and handling reflections.
    /// Returns `Ok(Some(true))` if the loop should continue, `Ok(Some(false))` if it should break,
    /// and `Ok(None)` if it should proceed normally.
    #[allow(clippy::too_many_arguments)]
    pub async fn process_llm_decision(
        &self,
        config: &SavedConfig,
        res: Box<LlmResponse>,
        img: &image::DynamicImage,
        zoomed_region: &Option<crate::vision::ZoomedRegion>,
        offsets: &[ScreenOffset],
        last_click_coordinates: &mut Option<(f64, f64)>,
        session_memory: &mut SessionMemory,
        loop_breaker: &mut LoopBreaker,
        escalate_to_desktop: &mut bool,
        task_graph: &mut Option<TaskGraph>,
        active_task_id: Option<TaskId>,
        reflection_feedback: &mut Option<String>,
        coordinate_calibration_retries: &mut u32,
        base_click_coordinates: &mut Option<(f64, f64)>,
    ) -> Result<Option<bool>, String> {
        self.bus.emit_notification(NotificationEvent::Log(format!(
            "📥 Received LLM Response | Action: '{}' | Status: '{}'",
            res.action, res.status_display
        )));

        let action_type = res.action.clone();
        let click_coords = if (action_type == "CLICK_AND_TYPE"
            || action_type == "RIGHT_CLICK"
            || action_type == "DOUBLE_CLICK"
            || action_type == "MIDDLE_CLICK")
            && res.relative_click_position.len() >= 2
        {
            let (mut rx, mut ry) = if let Some(ref zoomed) = zoomed_region {
                self.zoom_strategy.remap_coordinates((res.relative_click_position[0], res.relative_click_position[1]), zoomed)
            } else {
                (res.relative_click_position[0], res.relative_click_position[1])
            };

            let is_relative = rx <= 5.0 && ry <= 5.0;
            if is_relative {
                rx = rx.clamp(0.0, 1.0);
                ry = ry.clamp(0.0, 1.0);

                self.apply_coordinate_calibration(
                    config,
                    &mut rx,
                    &mut ry,
                    base_click_coordinates,
                    coordinate_calibration_retries,
                );
            }

            Some((rx, ry))
        } else {
            None
        };
        *last_click_coordinates = click_coords;

        let mut res = res;
        if let Some((rx, ry)) = click_coords {
            res.relative_click_position = vec![rx, ry];
        }

        // Build action key for loop detection
        let action_key = format!("{}:{:.2}:{:.2}:{}",
            res.action,
            res.relative_click_position.first().copied().unwrap_or(0.0),
            res.relative_click_position.get(1).copied().unwrap_or(0.0),
            res.text_to_type.len()
        );

        // Record into SessionMemory
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let was_repeated = action_key == loop_breaker.last_action_key;
        let step_to_record = ActionStep {
            timestamp,
            action_type: res.action.clone(),
            coordinates: click_coords,
            text_typed: if res.text_to_type.is_empty() { None } else { Some(res.text_to_type.clone()) },
            llm_report: res.report.clone().unwrap_or_default(),
            was_repeated,
        };
        session_memory.record(step_to_record.clone());

        // Log to incremental journal
        if let Some(ref journal) = self.journal {
            let entry = crate::append_store::JournalEntry {
                timestamp: chrono::Utc::now(),
                entry_type: crate::append_store::JournalEntryType::SessionStep(step_to_record),
            };
            let _ = journal.append(&entry);
        }

        // Compress history if option is enabled and steps >= 10
        if config.activer_compression_historique && session_memory.steps.len() >= 10 {
            self.compress_session_history(config, session_memory).await;
        } else {
            while session_memory.steps.len() > MAX_HISTORY_STEPS {
                session_memory.steps.pop_front();
            }
        }

        // Register with LoopBreaker and get escape strategy
        let loop_action = loop_breaker.register_action_precise(
            &action_key,
            &res.action,
            click_coords,
            &res.text_to_type,
            res.scroll_value,
            &res.keys_to_press,
        );

        if let Some(ref r) = res.report {
            if !r.is_empty() {
                self.bus.emit_notification(NotificationEvent::AppendReport(r.clone()));
            }
        }

        // Apply loop-breaker strategy to next iteration
        match loop_action {
            LoopAction::Normal => {
                if res.action == "FAIL" {
                    self.bus.emit_notification(NotificationEvent::Log("Explicit AI failure action. Triggering desktop capture escalation...".to_string()));
                    *escalate_to_desktop = true;
                } else if res.action != "WAIT" {
                    *escalate_to_desktop = false;
                }
                session_memory.mark_significant_change();
            },
            LoopAction::WarnLlm => {
                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "⚠️ Loop level 1: Same action repeated {} times. Warning injected into LLM prompt.",
                    loop_breaker.repetition_count()
                )));
            },
            LoopAction::EscalateToDesktop => {
                self.bus.emit_notification(NotificationEvent::Log(
                    "⚠️ Loop level 2: Escalating to full desktop capture...".to_string()
                ));
                *escalate_to_desktop = true;
            },
            _ => {
                *escalate_to_desktop = true;
            }
        }

        // TaskGraph: If action is FAIL, fail the task immediately
        if res.action == "FAIL" {
            if let Some(ref mut graph) = task_graph {
                if let Some(tid) = active_task_id {
                    graph.fail_task(tid, res.status_display.clone());
                    self.bus.emit_notification(NotificationEvent::Log(format!("❌ TaskGraph: Task {} failed: {}", tid.0, res.status_display)));
                }
            }
        }

        let handle_res = self.handle_decision(*res).await;

        if config.trace_actions_visuelles {
            if let Some((rx, ry)) = click_coords {
                if rx <= 1.0 && ry <= 1.0 {
                    if let Some(crop_bytes) = crate::orchestrator::helpers::create_action_crop(img, rx, ry) {
                        self.bus.emit_notification(NotificationEvent::AppendActionCrop(crop_bytes));
                    }
                }
            }
        }

        match handle_res {
            Ok(()) => {
                // If using TaskGraph and the action was SUCCESS, it means the current sub-task completed
                if action_type == "SUCCESS" {
                    if let Some(ref mut graph) = task_graph {
                        if let Some(tid) = active_task_id {
                            graph.complete_task(tid);
                            self.bus.emit_notification(NotificationEvent::Log(format!("✅ TaskGraph: Completed task {}.", tid.0)));
                            if !graph.is_complete() {
                                // More tasks to do! Do not stop the orchestrator loop yet.
                                self.cancelable_sleep(Duration::from_secs(3)).await;
                                return Ok(Some(true));
                            }
                        }
                    }
                    return Ok(Some(false)); // break
                }

                // Reflection: check outcome after executing action
                if config.activer_reflexion && (action_type == "CLICK_AND_TYPE" || action_type == "SCROLL") {
                    let after_img = if offsets.len() == 1 {
                        let target = &offsets[0];
                        self.capturer.capture_bbox(target.left, target.top, target.width, target.height)
                    } else {
                        self.capturer.capture_desktop()
                    }.or_else(|| self.capturer.capture_desktop());

                    if let Some(ref after_image) = after_img {
                        let was_retry = loop_breaker.repetition_count() > 0;
                        self.handle_reflection_evaluation(
                            config,
                            img,
                            after_image,
                            &action_type,
                            was_retry,
                            task_graph,
                            active_task_id,
                            reflection_feedback,
                            session_memory,
                        ).await;
                    }
                }
            },
            Err(ref e) if e == "STOP_SUCCESS" => {
                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        graph.complete_task(tid);
                        self.bus.emit_notification(NotificationEvent::Log(format!("✅ TaskGraph: Completed task {}.", tid.0)));
                        if !graph.is_complete() {
                            // More tasks to do! Do not stop the orchestrator loop yet.
                            self.cancelable_sleep(Duration::from_secs(3)).await;
                            return Ok(Some(true));
                        }
                    }
                }
                return Ok(Some(false)); // break
            },
            Err(e) => {
                self.bus.emit_notification(NotificationEvent::Log(format!("Error: {}", e)));
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "ERROR".to_string(),
                    display: format!("Execution error: {}", e),
                });

                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        if let Some(task) = graph.tasks.get_mut(&tid) {
                            task.attempts += 1;
                            self.bus.emit_notification(NotificationEvent::Log(format!(
                                "🔄 TaskGraph: Attempt {}/{} failed due to error. Retrying...",
                                task.attempts - 1, task.max_attempts
                            )));
                        }
                        if graph.check_and_fail_exhausted(tid) {
                            self.bus.emit_notification(NotificationEvent::Log(format!("❌ TaskGraph: Task {} failed due to execution error.", tid.0)));
                        }
                    }
                }

                self.cancelable_sleep(Duration::from_secs(5)).await;
            }
        }

        Ok(None)
    }
}
