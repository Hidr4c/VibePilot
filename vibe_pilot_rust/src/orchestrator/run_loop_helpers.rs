use tokio::time::{sleep, Duration};
use crate::event_bus::{NotificationEvent, QueryEvent};
use crate::orchestrator::VibePilotOrchestrator;
use crate::orchestrator::session_memory::{ActionStep, SessionMemory};
use crate::orchestrator::loop_breaker::{LoopBreaker, LoopAction};
use crate::orchestrator::helpers::{draw_click_marker};
use crate::memory::{TaskGraph, TaskId, TaskStatus};
use crate::reflection::{ReflectionModule, ReflectionVerdict};
use crate::llm_client::LlmResponse;
use crate::config::SavedConfig;
use crate::screen_capture::WindowAnchorResult;

impl VibePilotOrchestrator {
    /// Initializes or regenerates the TaskGraph plan from the LLM.
    pub async fn initialize_task_graph(&self, config: &SavedConfig) -> Option<TaskGraph> {
        self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
            "🧠 Génération du plan de tâches (TaskGraph)..."
        } else {
            "🧠 Generating task graph plan..."
        }.to_string()));

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

        match self.llm_client.decompose_objective(
            &config.objectif,
            &config.contexte,
            &config.task,
            target_url,
            target_model,
            auth_mode,
            auth_key,
            auth_login,
            auth_pass,
            timeout,
        ).await {
            Ok(json_response) => {
                match TaskGraph::from_llm_response(&json_response) {
                    Ok(mut graph) => {
                        graph.set_max_attempts(config.max_tentatives_par_tache);
                        self.bus.emit_notification(NotificationEvent::Log(format!(
                            "✅ TaskGraph successfully initialized with {} sub-tasks.",
                            graph.len()
                        )));
                        Some(graph)
                    }
                    Err(err) => {
                        self.bus.emit_notification(NotificationEvent::Log(format!(
                            "❌ Failed to parse TaskGraph JSON from LLM: {}. Response: {}",
                            err, json_response
                        )));
                        None
                    }
                }
            }
            Err(err) => {
                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "❌ Failed to decompose objective: {}",
                    err
                )));
                None
            }
        }
    }

    /// Selects the next active sub-task ID from the TaskGraph.
    pub fn select_active_task(
        &self,
        graph: &mut TaskGraph,
        active_task_id: &mut Option<TaskId>,
    ) -> bool {
        if graph.is_complete() {
            self.bus.emit_notification(NotificationEvent::Log("🎉 TaskGraph: All sub-tasks are Completed! Objective reached.".to_string()));
            self.bus.emit_notification(NotificationEvent::UpdateStatus {
                text: "SUCCESS".to_string(),
                color: "green".to_string(),
            });
            return false;
        }
        
        // 1. Check if there is already an active (InProgress) task
        if let Some(tid) = graph.current_task() {
            if let Some(task) = graph.get_task(tid) {
                if task.status == TaskStatus::InProgress {
                    *active_task_id = Some(tid);
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "🔄 TaskGraph: Continuing task {}: \"{}\" (Attempt {}/{})",
                        tid.0, task.description, task.attempts, task.max_attempts
                    )));
                }
            }
        }
        
        // 2. If no task is currently active, find the next ready task
        if active_task_id.is_none() {
            if let Some(task) = graph.next_ready_task().cloned() {
                let tid = task.id;
                graph.start_task(tid);
                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "🔄 TaskGraph: Working on task {}: \"{}\" (Attempt {}/{})",
                    tid.0, task.description, task.attempts + 1, task.max_attempts
                )));
                *active_task_id = Some(tid);
            }
        }
        
        if active_task_id.is_none() {
            self.bus.emit_notification(NotificationEvent::Log("⛔ TaskGraph: No ready tasks available (graph is blocked or failed). Ending loop.".to_string()));
            self.bus.emit_notification(NotificationEvent::UpdateStatus {
                text: "FAILED (Blocked)".to_string(),
                color: "red".to_string(),
            });
            return false;
        }

        true
    }


    /// Queries the LLM (or DecisionRouter) to obtain the next action decision.
    #[allow(clippy::too_many_arguments)]
    pub async fn get_llm_decision(
        &self,
        config: &SavedConfig,
        img: &image::DynamicImage,
        final_image: &image::DynamicImage,
        _zoomed_region: &Option<crate::vision::ZoomedRegion>,
        session_memory: &SessionMemory,
        task_graph: &Option<TaskGraph>,
        active_task_id: Option<TaskId>,
        reflection_feedback: &Option<String>,
        loop_breaker: &LoopBreaker,
        last_click_coordinates: Option<(f64, f64)>,
        _escalate_to_desktop: bool,
        _offsets: &[crate::peripheral_controller::ScreenOffset],
        last_captured_image: &Option<image::DynamicImage>,
        is_specific_window: bool,
    ) -> Result<Box<LlmResponse>, String> {
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

        let mut response = Err("No route".to_string());
        let mut routed_fast = false;

        if config.activer_systeme_fast_slow {
            let d_router = crate::decision_router::DecisionRouter::new(self.capturer.clone());
            let d_context = crate::decision_router::DecisionContext {
                screenshot: img,
                last_screenshot: last_captured_image.as_ref(),
                last_action: session_memory.steps.back(),
                current_subtask_description: active_task_id.and_then(|tid| {
                    task_graph.as_ref().and_then(|g| g.get_task(tid).map(|t| t.description.as_str()))
                }),
                target_window_title: if is_specific_window { Some(&config.fenetres_surveillees[0]) } else { None },
                langue: &config.langue,
            };

            match d_router.route(&d_context) {
                crate::decision_router::DecisionRoute::Fast(res) => {
                    self.bus.emit_notification(NotificationEvent::Log(if config.langue == "Français" {
                        "⚡ [Fast Route] Action résolue localement via heuristique."
                    } else {
                        "⚡ [Fast Route] Action resolved locally via heuristics."
                    }.to_string()));
                    response = Ok(res);
                    routed_fast = true;
                }
                crate::decision_router::DecisionRoute::Slow => {}
            }
        }

        if !routed_fast {
            let user_feedback = self.bus.emit_query(QueryEvent::GetUserFeedback);
            if !user_feedback.is_empty() {
                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "💬 [Feedback User] prise en compte de l'indication : \"{}\"",
                    user_feedback
                )));
            }

            let mut custom_directives = config.directives.clone();

            if loop_breaker.repetition_count() >= 1 {
                let warning = if config.langue == "Français" {
                    format!("\n⚠️ MISE EN GARDE DE BOUCLE : Vous avez répété la même action précédente {} fois sans modification de l'écran.\nVous DEVEZ appliquer la MÉTHODE DE CORRECTION AXE PAR AXE pour ajuster vos coordonnées :\n- Ajustez l'axe X d'abord (en gardant Y constant).\n- Si cela ne fonctionne pas, ajustez l'axe Y (en gardant X constant).\n- Alternez l'ajustement entre X et Y pour cibler précisément l'élément.\nNe répétez pas la même coordonnée !", loop_breaker.repetition_count() + 1)
                } else {
                    format!("\n⚠️ LOOP DETECTION WARNING: You have repeated the exact same action {} times with no visual changes on screen.\nYou MUST apply the AXIS-BY-AXIS CORRECTION METHOD to adjust your coordinates:\n- Adjust the X axis first (keeping Y constant).\n- If that does not work, adjust the Y axis (keeping X constant).\n- Alternate adjustments between X and Y to target the element precisely.\nDo not repeat the same coordinate!", loop_breaker.repetition_count() + 1)
                };
                custom_directives = format!("{}\n{}", custom_directives, warning);
            }

            if let Some((cx, cy)) = last_click_coordinates {
                let note = if config.langue == "Français" {
                    format!("\nℹ️ NOTE VISUELLE : Votre clic précédent a été effectué aux coordonnées relatives x={:.3}, y={:.3} (marqué par une CROIX ROUGE et un CERCLE ROUGE sur la capture). Utilisez ce repère pour ajuster vos coordonnées si le clic a raté !\n👉 MÉTHODE DE CORRECTION AXE PAR AXE :\n1. Si le clic précédent à (x, y) n'a rien produit, NE répétez PAS les mêmes coordonnées.\n2. Ajustez d'abord l'axe X (gauche/droite) tout en gardant l'axe Y constant.\n3. Si l'axe X semble correct mais que le clic échoue toujours, gardez X constant et ajustez l'axe Y (haut/bas).\n4. Si cela échoue encore, alternez à nouveau et ré-ajustez l'axe X.\n5. Expliquez clairement dans votre champ \"report\" quel axe vous êtes en train d'ajuster (ex: \"Ajustement de l'axe X en gardant Y constant\").", cx, cy)
                } else {
                    format!("\nℹ️ VISUAL NOTE: Your previous click was performed at relative coordinates x={:.3}, y={:.3} (indicated by a RED CROSSHAIR and RED CIRCLE on the screenshot). Use this marker to adjust your coordinates if the previous click missed the target!\n👉 AXIS-BY-AXIS CORRECTION METHOD:\n1. If the previous click at (x, y) had no effect, DO NOT repeat the exact same coordinates.\n2. First adjust the X axis (left/right) while keeping the Y axis constant.\n3. If X seems correct but the click still fails, keep X constant and adjust the Y axis (up/down).\n4. If it still fails, alternate back to adjusting the X axis.\n5. State clearly in your \"report\" field which axis you are currently adjusting (e.g., \"Adjusting X axis while keeping Y constant\").", cx, cy)
                };
                custom_directives = format!("{}\n{}", custom_directives, note);
            }

            // structured session memory
            let memory_context = session_memory.format_for_prompt(&config.langue);
            if !memory_context.is_empty() {
                custom_directives = format!("{}{}", custom_directives, memory_context);
            }

            // structured task graph context
            if let Some(ref graph) = task_graph {
                let graph_prompt = graph.format_for_prompt(&config.langue);
                custom_directives = format!("{}{}", custom_directives, graph_prompt);
            }

            // visual reflection feedback
            if let Some(ref feedback) = reflection_feedback {
                let warning = if config.langue == "Français" {
                    format!("\n\n⚠️ RETOUR DE RÉFLEXION : {}", feedback)
                } else {
                    format!("\n\n⚠️ REFLECTION FEEDBACK: {}", feedback)
                };
                custom_directives = format!("{}{}", custom_directives, warning);
            }
            // Append Monitored Window Accessibility Layout Tree
            if let Ok(tree_str) = self.accessibility_parser.parse_active_window() {
                custom_directives = format!("{}\n\n{}", custom_directives, tree_str);
            }

            let primary_res = self.llm_client.execute_decision(
                final_image,
                &config.contexte,
                &config.objectif,
                &config.task,
                &custom_directives,
                &user_feedback,
                target_url,
                target_model,
                auth_mode,
                auth_key,
                auth_login,
                auth_pass,
                timeout,
            ).await;

            response = match primary_res {
                Ok(resp) => Ok(Box::new(resp)),
                Err(err) => {
                    self.bus.emit_notification(crate::event_bus::NotificationEvent::Log(format!(
                        "Primary LLM API failed: {}. Retrying with local Ollama fallback...",
                        err
                    )));
                    
                    let ollama_url = "http://127.0.0.1:11434/v1/chat/completions";
                    let ollama_model = "llava";
                    let fallback_res = self.llm_client.execute_decision(
                        final_image,
                        &config.contexte,
                        &config.objectif,
                        &config.task,
                        &custom_directives,
                        &user_feedback,
                        ollama_url,
                        ollama_model,
                        "none",
                        "",
                        "",
                        "",
                        timeout,
                    ).await;
                    
                    match fallback_res {
                        Ok(resp) => {
                            self.bus.emit_notification(crate::event_bus::NotificationEvent::Log(
                                "Local Ollama fallback succeeded!".to_string()
                            ));
                            Ok(Box::new(resp))
                        }
                        Err(fallback_err) => {
                            Err(format!("Primary LLM failed: {}. Ollama fallback failed: {}", err, fallback_err))
                        }
                    }
                }
            };
        }

        response
    }


    /// Performs visual reflection and evaluation of action outcomes.
    pub async fn handle_reflection_evaluation(
        &self,
        _config: &SavedConfig,
        before_img: &image::DynamicImage,
        after_img: &image::DynamicImage,
        action_type: &str,
        was_retry: bool,
        task_graph: &mut Option<TaskGraph>,
        active_task_id: Option<TaskId>,
        reflection_feedback: &mut Option<String>,
        session_memory: &mut SessionMemory,
    ) {
        let diff_result = ReflectionModule::quick_visual_diff(before_img, after_img);
        let verdict = ReflectionModule::evaluate_local(&diff_result, action_type, was_retry);
        self.bus.emit_notification(NotificationEvent::Log(format!("💭 Reflection verdict: {:?}", verdict)));

        match verdict {
            ReflectionVerdict::NoEffect { suggestion } => {
                self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Action had no effect! Suggestion: {}", suggestion)));
                *reflection_feedback = Some(suggestion.clone());

                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        graph.set_reflection(tid, suggestion);
                        if let Some(task) = graph.tasks.get_mut(&tid) {
                            task.attempts += 1;
                            self.bus.emit_notification(NotificationEvent::Log(format!(
                                "🔄 TaskGraph: Attempt {}/{} failed. Retrying task {}...",
                                task.attempts - 1, task.max_attempts, tid.0
                            )));
                        }
                        if graph.check_and_fail_exhausted(tid) {
                            self.bus.emit_notification(NotificationEvent::Log(format!("❌ TaskGraph: Task {} failed after exhausting maximum attempts.", tid.0)));
                        }
                    }
                }
            }
            ReflectionVerdict::Success { confidence } => {
                self.bus.emit_notification(NotificationEvent::Log(format!("✅ Action succeeded (confidence: {:.2})", confidence)));
                session_memory.mark_significant_change();
                *reflection_feedback = None;

                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        graph.set_reflection(tid, format!("Action succeeded (confidence: {:.2})", confidence));
                    }
                }
            }
            ReflectionVerdict::UnexpectedChange { description } => {
                self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Unexpected change detected: {}", description)));
                *reflection_feedback = Some(description.clone());

                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        graph.set_reflection(tid, description);
                    }
                }
            }
            ReflectionVerdict::Regression { rollback_suggestion } => {
                self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Regression detected! Rollback suggestion: {}", rollback_suggestion)));
                *reflection_feedback = Some(rollback_suggestion.clone());

                if let Some(ref mut graph) = task_graph {
                    if let Some(tid) = active_task_id {
                        graph.set_reflection(tid, rollback_suggestion);
                    }
                }
            }
        }
    }

    /// Checks iteration count and session duration safety limits.
    pub fn check_safety_limits(&self, iteration_count: u64, session_start: tokio::time::Instant) -> Result<(), String> {
        const MAX_ITERATIONS: u64 = 1000;
        const MAX_SESSION_DURATION_SECS: u64 = 3600;

        if iteration_count > MAX_ITERATIONS {
            self.bus.emit_notification(NotificationEvent::Log(format!(
                "Stopping: reached maximum iterations ({})", MAX_ITERATIONS
            )));
            return Err(format!("Maximum iterations ({}) reached", MAX_ITERATIONS));
        }

        let elapsed = session_start.elapsed().as_secs();
        if elapsed > MAX_SESSION_DURATION_SECS {
            self.bus.emit_notification(NotificationEvent::Log(format!(
                "Stopping: session timeout ({:.0}s > {:.0}s)",
                elapsed, MAX_SESSION_DURATION_SECS
            )));
            return Err(format!("Session timeout after {:.0} seconds", elapsed));
        }
        Ok(())
    }

    /// Executes the Visual Re-Anchoring Protocol.
    /// Returns `true` if the main loop should restart the iteration (`continue`).
    pub async fn run_window_reanchoring(
        &self,
        config: &SavedConfig,
        escalate_to_desktop: &mut bool,
        window_recovery_attempted: &mut bool,
    ) -> bool {
        let is_specific_window = !*escalate_to_desktop
            && !config.fenetres_surveillees.is_empty()
            && !self.controller.is_desktop_title(&config.fenetres_surveillees[0]);

        if is_specific_window {
            // If the user has focused the VibePilot window, do not steal focus back
            if let Some(fg_title) = self.capturer.get_foreground_window_title() {
                if fg_title.contains("VibePilot") {
                    sleep(Duration::from_millis(300)).await;
                    return true; // continue outer loop
                }
            }

            let target_title = &config.fenetres_surveillees[0];
            let anchor_result = self.capturer.ensure_window_foreground(target_title);
            match anchor_result {
                WindowAnchorResult::AlreadyFocused => {
                    *window_recovery_attempted = false;
                },
                WindowAnchorResult::Refocused => {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "🔄 Re-anchored: target window '{}' was not in foreground, brought back to focus.",
                        target_title
                    )));
                    *window_recovery_attempted = false;
                    // Small extra delay after re-focus for the window to render
                    sleep(Duration::from_millis(200)).await;
                },
                WindowAnchorResult::WindowNotFound => {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "⚠️ Window '{}' not found! Attempting recovery...", target_title
                    )));
                    if !*window_recovery_attempted {
                        *window_recovery_attempted = true;
                        let recovered = self.attempt_window_recovery(target_title).await;
                        if !recovered {
                            self.bus.emit_notification(NotificationEvent::Log(
                                "❌ Window recovery failed. Escalating to desktop capture.".to_string()
                            ));
                            *escalate_to_desktop = true;
                        } else {
                            self.bus.emit_notification(NotificationEvent::Log(
                                "✅ Window recovered successfully!".to_string()
                            ));
                            return true; // continue outer loop
                        }
                    } else {
                        *escalate_to_desktop = true;
                    }
                }
            }
        }
        false
    }
}

