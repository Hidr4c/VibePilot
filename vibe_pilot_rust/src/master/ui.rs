use egui::{Color32, Rect, RichText, Vec2};
use super::MasterApp;
use super::dag::{GridNode, NodeState};

/// Renders the master control dashboard.
pub fn render_master_gui(ctx: &egui::Context, app: &mut MasterApp) {
    // Premium Dark Theme styling setup
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(Color32::from_rgb(230, 235, 240));
    ctx.set_style(style);

    egui::CentralPanel::default().show(ctx, |ui| {
        // 1. Premium Gradient Header Bar
        render_header(ui);

        ui.add_space(8.0);

        ui.columns(2, |columns| {
            // Left Column: Node Configuration & Prompter Panel (350.0 width)
            let left_ui = &mut columns[0];
            left_ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("🧠 VibePilot Compiler").strong().color(Color32::from_rgb(0, 180, 216)));
                        ui.label("Describe your workflow in natural language:");
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut app.llm_prompt)
                                .hint_text("e.g. Start local worker at port 4040, open paint, then send teams notification...")
                                .desired_rows(4)
                                .desired_width(f32::INFINITY)
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("⚡ Compile DAG").strong().color(Color32::from_rgb(0, 119, 182))).clicked() {
                                app.message = "Analyzing prompt...".to_string();
                                compile_dag_via_prompt(ctx, app);
                            }
                            if app.is_compiling {
                                ui.spinner();
                            }
                        });
                    });
                });

                ui.add_space(8.0);

                // Selected Node Properties Editor
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("⚙ Node Properties").strong().color(Color32::from_rgb(144, 224, 239)));
                        let mut needs_reschedule = false;
                        let mut graph_lock = app.graph.lock().unwrap();
                        if let Some(ref selected_id) = app.selected_node_id {
                            if let Some(node) = graph_lock.nodes.get_mut(selected_id) {
                                ui.horizontal(|ui| {
                                    ui.label("Node ID: ");
                                    ui.monospace(selected_id);
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label("Name: ");
                                    if ui.text_edit_singleline(&mut node.name).changed() {
                                        needs_reschedule = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Profile: ");
                                    ui.text_edit_singleline(&mut node.profile_name);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Endpoint: ");
                                    if ui.text_edit_singleline(&mut node.worker_url).changed() {
                                        if let Ok(mut s) = app.worker_statuses.lock() {
                                            s.remove(&node.worker_url);
                                        }
                                    }
                                    
                                    let status = {
                                        let mut statuses = app.worker_statuses.lock().unwrap();
                                        statuses.entry(node.worker_url.clone()).or_insert_with(|| "Unknown".to_string()).clone()
                                    };
                                    
                                    let color = match status.as_str() {
                                        "Online" => Color32::GREEN,
                                        "Offline" => Color32::RED,
                                        "Checking..." => Color32::from_rgb(255, 165, 0),
                                        _ => Color32::GRAY,
                                    };
                                    
                                    ui.label(RichText::new(format!("● {}", status)).color(color));
                                    
                                    if status == "Unknown" || ui.button("⟳").clicked() {
                                        let url = node.worker_url.clone();
                                        let statuses = app.worker_statuses.clone();
                                        {
                                            let mut s = statuses.lock().unwrap();
                                            s.insert(url.clone(), "Checking...".to_string());
                                        }
                                        let rt = app.rt.handle().clone();
                                        rt.spawn(async move {
                                            let check_res = query_status_check(url.clone()).await;
                                            let mut s = statuses.lock().unwrap();
                                            s.insert(url, check_res);
                                        });
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Duration (s): ");
                                    let mut dur = node.duration_secs;
                                    if ui.add(egui::Slider::new(&mut dur, 1.0..=120.0).text("")).changed() {
                                        node.duration_secs = dur;
                                        needs_reschedule = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("State: ");
                                    egui::ComboBox::from_label("")
                                        .selected_text(format!("{:?}", node.state))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut node.state, NodeState::Pending, "Pending");
                                            ui.selectable_value(&mut node.state, NodeState::Running, "Running");
                                            ui.selectable_value(&mut node.state, NodeState::Success, "Success");
                                            ui.selectable_value(&mut node.state, NodeState::Failed, "Failed");
                                        });
                                });
                                
                                // Live logs section for selected node
                                ui.add_space(8.0);
                                ui.separator();
                                ui.label(RichText::new("📝 Node Logs (Live)").strong().color(Color32::from_rgb(0, 180, 216)));
                                egui::ScrollArea::vertical()
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        if let Ok(logs_map) = app.node_logs.lock() {
                                            if let Some(logs) = logs_map.get(selected_id) {
                                                for log in logs {
                                                    ui.label(RichText::new(log).monospace().size(10.0));
                                                }
                                            } else {
                                                ui.label(RichText::new("No logs received from worker.").weak().italics());
                                            }
                                        }
                                    });
                            }
                        } else {
                            ui.label("Select a node on the Gantt timeline to edit its settings.");
                        }
                        if needs_reschedule {
                            graph_lock.compute_schedule();
                        }
                    });
                });
            });

            // Right Column: Interactive Gantt Chart Scheduler & State Viewer
            let right_ui = &mut columns[1];
            right_ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(RichText::new("📊 Gantt Dependency Timeline").strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new("▶ Execute Grid").color(Color32::GREEN)).clicked() {
                                    app.message = "Launching grid execution loop...".to_string();
                                    let executor = super::executor::GridGraphExecutor::new(
                                        app.graph.clone(),
                                        app.node_logs.clone(),
                                    );
                                    let rt_handle = app.rt.handle();
                                    executor.start_execution(rt_handle);
                                }
                            });
                        });
                        ui.add_space(8.0);
                        
                        render_gantt_chart(ui, app);
                    });
                });
                
                ui.add_space(8.0);
                
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Console Log:").strong().color(Color32::from_rgb(0, 180, 216)));
                        ui.label(&app.message);
                    });
                });
            });
        });
    });
}

fn render_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("🌐 VibePilot Grid").font(egui::FontId::proportional(22.0)).strong().color(Color32::from_rgb(144, 224, 239)));
        ui.label(RichText::new("MASTER SYSTEM").font(egui::FontId::monospace(12.0)).color(Color32::GRAY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("v1.0.0").small());
        });
    });
    ui.separator();
}

pub(crate) fn compile_dag_via_prompt(ctx: &egui::Context, app: &mut MasterApp) {
    let config = app.config_repo.load_config();
    let prompt = app.llm_prompt.clone();
    let url = config.url_api.clone();
    let model = config.nom_modele.clone();
    let auth_mode = config.auth_mode.clone();
    let auth_api_key = config.auth_api_key.clone();
    let auth_login = config.auth_login.clone();
    let auth_password = config.auth_password.clone();
    let timeout_secs = config.request_timeout_secs;

    let sender = app.compiler_tx.clone();
    let ctx_clone = ctx.clone();
    app.is_compiling = true;
    app.message = "Compiling DAG with LLM...".to_string();

    app.rt.spawn(async move {
        let res = super::prompt_compiler::LlmPromptCompiler::compile(
            &prompt,
            &url,
            &model,
            &auth_mode,
            &auth_api_key,
            &auth_login,
            &auth_password,
            timeout_secs,
        ).await;
        let _ = sender.send(res);
        ctx_clone.request_repaint();
    });
}

fn render_gantt_chart(ui: &mut egui::Ui, app: &mut MasterApp) {
    let row_height = 40.0;
    let time_scale = 3.0; // pixels per second
    let margin_left = 120.0;

    let graph_lock = app.graph.lock().unwrap();
    let total_nodes = graph_lock.nodes.len();
    if total_nodes == 0 {
        ui.label("No nodes present in DAG.");
        return;
    }

    let desired_size = Vec2::new(ui.available_width(), (total_nodes as f32) * row_height + 40.0);
    
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    
    let painter = ui.painter_at(rect);
    
    // Draw background grid lines
    painter.rect_filled(rect, 4.0, Color32::from_rgb(30, 35, 45));
    
    // Draw time scale headers (e.g., 0s, 10s, 20s, 30s)
    for i in 0..15 {
        let sec = i * 10;
        let x = rect.left() + margin_left + (sec as f32) * time_scale;
        if x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top() + 20.0), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(0.5, Color32::from_rgb(60, 65, 75))
            );
            painter.text(
                egui::pos2(x, rect.top() + 15.0),
                egui::Align2::CENTER_CENTER,
                format!("{}s", sec),
                egui::FontId::monospace(10.0),
                Color32::GRAY
            );
        }
    }

    // Sort nodes by start offset to render consistently
    let mut sorted_nodes: Vec<&GridNode> = graph_lock.nodes.values().collect();
    sorted_nodes.sort_by(|a, b| a.start_time_offset.partial_cmp(&b.start_time_offset).unwrap_or(std::cmp::Ordering::Equal));

    for (idx, node) in sorted_nodes.iter().enumerate() {
        let y_pos = rect.top() + 30.0 + (idx as f32) * row_height;
        
        // Render Node Label
        painter.text(
            egui::pos2(rect.left() + 10.0, y_pos + 10.0),
            egui::Align2::LEFT_CENTER,
            &node.name,
            egui::FontId::proportional(12.0),
            Color32::from_rgb(200, 210, 220)
        );

        // Calculate Gantt bar rect
        let bar_left = rect.left() + margin_left + node.start_time_offset * time_scale;
        let bar_right = bar_left + node.duration_secs * time_scale;
        let bar_rect = Rect::from_min_max(
            egui::pos2(bar_left, y_pos),
            egui::pos2(bar_right, y_pos + 20.0)
        );

        // Determine bar color based on execution state
        let color = match node.state {
            NodeState::Pending => Color32::from_rgb(70, 80, 95),
            NodeState::Running => Color32::from_rgb(0, 119, 182),
            NodeState::Success => Color32::from_rgb(46, 117, 89),
            NodeState::Failed => Color32::from_rgb(186, 45, 45),
        };

        // Render glassmorphism highlights if selected
        let is_selected = app.selected_node_id.as_ref() == Some(&node.id);
        painter.rect_filled(bar_rect, 4.0, color);
        if is_selected {
            let stroke = egui::Stroke::new(2.0, Color32::from_rgb(144, 224, 239));
            painter.rect_stroke(bar_rect, 4.0, stroke, egui::StrokeKind::Inside);
        }

        // Click detection on Gantt bars
        if response.clicked() {
            if let Some(pointer_pos) = response.hover_pos() {
                if bar_rect.contains(pointer_pos) {
                    app.selected_node_id = Some(node.id.clone());
                }
            }
        }
    }
}

async fn query_status_check(worker_url: String) -> String {
    let (host_port, token) = crate::master::parse_worker_url(&worker_url);
    if host_port.is_empty() {
        return "Offline".to_string();
    }
    
    let connect_fut = tokio::net::TcpStream::connect(&host_port);
    match tokio::time::timeout(std::time::Duration::from_millis(800), connect_fut).await {
        Ok(Ok(mut stream)) => {
            use tokio::io::{AsyncWriteExt, AsyncReadExt};
            let mut auth_header = String::new();
            if let Some(ref t) = token {
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let sig = crate::master::generate_signature(t, timestamp, "/status");
                auth_header = format!("X-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n", timestamp, sig);
            }
            let request = format!("GET /status HTTP/1.1\r\n{}Connection: close\r\n\r\n", auth_header);
            if stream.write_all(request.as_bytes()).await.is_ok() {
                let mut resp = String::new();
                let read_fut = stream.read_to_string(&mut resp);
                if tokio::time::timeout(std::time::Duration::from_millis(800), read_fut).await.is_ok() {
                    if resp.contains("Running") || resp.contains("Paused") || resp.contains("Idle") {
                        return "Online".to_string();
                    }
                }
            }
        }
        _ => {}
    }
    "Offline".to_string()
}
