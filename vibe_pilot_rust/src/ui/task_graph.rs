use eframe::egui;
use std::collections::{HashMap, HashSet};
use crate::app::{VibePilotApp, StructuredStep};
use crate::memory::{TaskId, TaskNode, TaskStatus};
use crate::event_bus::NotificationEvent;

fn get_steps_for_task(id: TaskId, steps: &[StructuredStep]) -> Vec<(usize, StructuredStep)> {
    let mut task_steps = Vec::new();
    let prefix_working = format!("TaskGraph: Working on task {}", id.0);
    let prefix_continuing = format!("TaskGraph: Continuing task {}", id.0);
    let prefix_completed = format!("TaskGraph: Completed task {}", id.0);
    
    for (idx, step) in steps.iter().enumerate() {
        let mut belongs_to_task = false;
        for log in &step.logs {
            if log.contains(&prefix_working) || log.contains(&prefix_continuing) || log.contains(&prefix_completed) {
                belongs_to_task = true;
                break;
            }
        }
        if belongs_to_task {
            task_steps.push((idx, step.clone()));
        }
    }
    task_steps
}

pub fn render_task_graph_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.vertical(|ui| {
        ui.heading("🪵 Task Graph Visualizer & Editor");
        ui.label("Here you can inspect the current AI plan, visualize task dependencies, and manually modify the graph.");
        ui.separator();

        // Load the current task graph from the configuration repository
        let mut graph_opt = app.config_repo.load_task_graph();
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Reload Graph").clicked() {
                app.bus.emit_notification(NotificationEvent::Log("Task graph reloaded".to_string()));
            }
            if ui.button("🧹 Clear Graph").clicked() {
                app.config_repo.save_task_graph(None);
                app.bus.emit_notification(NotificationEvent::Log("Task graph cleared".to_string()));
                graph_opt = None;
            }
        });

        ui.add_space(8.0);

        if let Some(mut graph) = graph_opt {
            let replay_lbl = app.t("replay_from_here");
            let restart_lbl = app.t("restart_step");
            let complete_lbl = app.t("mark_completed");
            let pending_lbl = app.t("mark_pending");
            let mut graph_modified = None;

            // --- Editor Tools (Top Section) ---
            ui.group(|ui| {
                ui.label(egui::RichText::new("🛠️ Editor Tools").strong());
                ui.separator();
                ui.columns(3, |columns| {
                    // Column 1: Add New Task
                    columns[0].vertical(|ui| {
                        ui.label(egui::RichText::new("➕ Add New Task").strong());
                        ui.horizontal(|ui| {
                            ui.label("Desc:");
                            ui.text_edit_singleline(&mut app.user_feedback_input);
                        });
                        if ui.button("Add Task").clicked() {
                            let desc = app.user_feedback_input.trim().to_string();
                            if !desc.is_empty() {
                                let next_id = TaskId(graph.next_id);
                                let new_node = TaskNode::new(next_id, desc, vec![]);
                                graph.add_task(new_node);
                                app.config_repo.save_task_graph(Some(graph.clone()));
                                app.user_feedback_input = String::new();
                                app.bus.emit_notification(NotificationEvent::Log(format!("Added task #{}", next_id.0)));
                            }
                        }
                    });

                    // Column 2: Add Dependency
                    columns[1].vertical(|ui| {
                        ui.label(egui::RichText::new("🔗 Add Dependency").strong());
                        ui.horizontal(|ui| {
                            ui.label("Child ID:");
                            ui.add(egui::TextEdit::singleline(&mut app.selected_profile_to_export).desired_width(40.0));
                            ui.label("Parent ID:");
                            ui.add(egui::TextEdit::singleline(&mut app.selected_engine_to_export).desired_width(40.0));
                        });
                        if ui.button("Connect").clicked() {
                            let child_raw: Result<u64, _> = app.selected_profile_to_export.trim().parse();
                            let parent_raw: Result<u64, _> = app.selected_engine_to_export.trim().parse();
                            if let (Ok(child_val), Ok(parent_val)) = (child_raw, parent_raw) {
                                let child = TaskId(child_val);
                                let parent = TaskId(parent_val);
                                if graph.tasks.contains_key(&child) && graph.tasks.contains_key(&parent) {
                                    if let Some(node) = graph.tasks.get_mut(&child) {
                                        if !node.dependencies.contains(&parent) {
                                            node.dependencies.push(parent);
                                        }
                                    }
                                    if graph.has_cycle() {
                                        app.bus.emit_notification(NotificationEvent::Log("⚠️ Cycle detected!".to_string()));
                                        if let Some(node) = graph.tasks.get_mut(&child) {
                                            node.dependencies.retain(|&x| x != parent);
                                        }
                                    } else {
                                        app.config_repo.save_task_graph(Some(graph.clone()));
                                        app.bus.emit_notification(NotificationEvent::Log(format!("Connected Task #{} to #{}", child.0, parent.0)));
                                    }
                                }
                            }
                        }
                    });

                    // Column 3: Delete Tasks
                    columns[2].vertical(|ui| {
                        ui.label(egui::RichText::new("🗑️ Delete Tasks").strong());
                        egui::ScrollArea::vertical().id_salt("delete_tasks_scroll").max_height(70.0).show(ui, |ui| {
                            let tasks_info: Vec<(TaskId, String)> = graph.tasks.iter()
                                .map(|(&id, node)| (id, node.description.clone()))
                                .collect();
                            for (id, desc) in tasks_info {
                                ui.horizontal(|ui| {
                                    ui.label(format!("#{} - {}", id.0, &desc[..desc.len().min(12)]));
                                    if ui.button("❌").clicked() {
                                        graph.tasks.remove(&id);
                                        graph.root_order.retain(|&x| x != id);
                                        for other_node in graph.tasks.values_mut() {
                                            other_node.dependencies.retain(|&x| x != id);
                                        }
                                        app.config_repo.save_task_graph(Some(graph.clone()));
                                        app.bus.emit_notification(NotificationEvent::Log(format!("Deleted task #{}", id.0)));
                                    }
                                });
                            }
                        });
                    });
                });
            });

            ui.add_space(8.0);

            // --- Visualizer (Middle Section) ---
            ui.group(|ui| {
                ui.subheader("Visualizer");

                if graph.tasks.is_empty() {
                    let size = egui::vec2(ui.available_width(), 200.0);
                    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
                    let rect = response.rect;
                    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Empty task graph",
                        egui::FontId::monospace(14.0),
                        ui.visuals().text_color(),
                    );
                } else {
                    // Compute layers based on dependencies
                    let mut layers: HashMap<TaskId, usize> = HashMap::new();
                    let mut visited = HashSet::new();

                    // Helper to compute node depth
                    fn get_depth(
                        id: TaskId,
                        tasks: &HashMap<TaskId, TaskNode>,
                        layers: &mut HashMap<TaskId, usize>,
                        visited: &mut HashSet<TaskId>,
                    ) -> usize {
                        if let Some(depth) = layers.get(&id) {
                            return *depth;
                        }
                        if visited.contains(&id) {
                            return 0;
                        }
                        visited.insert(id);

                        let mut max_dep_depth = 0;
                        if let Some(node) = tasks.get(&id) {
                            for dep in &node.dependencies {
                                let dep_depth = get_depth(*dep, tasks, layers, visited);
                                if dep_depth + 1 > max_dep_depth {
                                    max_dep_depth = dep_depth + 1;
                                }
                            }
                        }

                        visited.remove(&id);
                        layers.insert(id, max_dep_depth);
                        max_dep_depth
                    }

                    for id in graph.tasks.keys() {
                        get_depth(*id, &graph.tasks, &mut layers, &mut visited);
                    }

                    // Group tasks by layer
                    let mut layer_groups: HashMap<usize, Vec<TaskId>> = HashMap::new();
                    for (id, layer) in &layers {
                        layer_groups.entry(*layer).or_default().push(*id);
                    }

                    let max_layer = layer_groups.keys().max().copied().unwrap_or(0);
                    let layer_height = 80.0;
                    let total_height = (max_layer + 1) as f32 * layer_height + 40.0;

                    let max_nodes_in_layer = layer_groups.values().map(|v| v.len()).max().unwrap_or(1);
                    let total_width = ui.available_width().max(max_nodes_in_layer as f32 * 300.0);

                    let size = egui::vec2(total_width, total_height);

                    // Wrap visualizer in ScrollArea
                    egui::ScrollArea::both()
                        .id_salt("visualizer_scroll")
                        .max_height(280.0)
                        .show(ui, |ui| {
                            let (response, painter) = ui.allocate_painter(size, egui::Sense::click() | egui::Sense::hover());
                            let rect = response.rect;
                            painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

                            // Compute node coordinates
                            let padding_x = 50.0;
                            let mut coords: HashMap<TaskId, egui::Pos2> = HashMap::new();
                            for (layer, ids) in &layer_groups {
                                let y = rect.top() + 30.0 + (*layer as f32 * layer_height);
                                for (index, id) in ids.iter().enumerate() {
                                     let x = rect.left() + padding_x + (index as f32 * 300.0);
                                     coords.insert(*id, egui::pos2(x, y));
                                }
                            }

                            // Draw dependency lines
                            for (id, node) in &graph.tasks {
                                if let Some(child_pos) = coords.get(id) {
                                    for dep_id in &node.dependencies {
                                        if let Some(parent_pos) = coords.get(dep_id) {
                                            painter.line_segment(
                                                [*parent_pos, *child_pos],
                                                egui::Stroke::new(2.0, egui::Color32::from_gray(120)),
                                            );
                                        }
                                    }
                                }
                            }

                            // Draw node circles
                            for (id, node) in &graph.tasks {
                                if let Some(pos) = coords.get(id) {
                                    let (color, label_prefix) = match &node.status {
                                        TaskStatus::Pending => (egui::Color32::from_rgb(100, 149, 237), "P"),
                                        TaskStatus::InProgress => (egui::Color32::from_rgb(255, 165, 0), "I"),
                                        TaskStatus::Completed => (egui::Color32::from_rgb(50, 205, 50), "C"),
                                        TaskStatus::Failed { .. } => (egui::Color32::from_rgb(220, 20, 60), "F"),
                                        TaskStatus::Blocked => (egui::Color32::from_rgb(169, 169, 169), "B"),
                                        TaskStatus::Skipped => (egui::Color32::from_rgb(120, 120, 200), "S"),
                                    };

                                    painter.circle_filled(*pos, 16.0, color);
                                    painter.circle_stroke(*pos, 16.0, egui::Stroke::new(2.0, egui::Color32::WHITE));

                                    painter.text(
                                        *pos,
                                        egui::Align2::CENTER_CENTER,
                                        label_prefix,
                                        egui::FontId::monospace(14.0),
                                        egui::Color32::WHITE,
                                    );

                                    painter.text(
                                        *pos + egui::vec2(24.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        format!("#{} - {}", id.0, node.description),
                                        egui::FontId::proportional(12.0),
                                        ui.visuals().text_color(),
                                    );

                                    let node_rect = egui::Rect::from_center_size(*pos, egui::vec2(32.0, 32.0));
                                    let node_id = ui.make_persistent_id(format!("task_node_{}", id.0));
                                    let node_clone = node.clone();
                                    let id_val = *id;

                                    let node_response = ui.interact(node_rect, node_id, egui::Sense::click() | egui::Sense::hover())
                                        .on_hover_ui(|ui| {
                                            ui.heading(format!("Task #{} details", id_val.0));
                                            ui.label(format!("Description: {}", node_clone.description));
                                            ui.label(format!("Status: {:?}", node_clone.status));
                                            ui.label(format!("Attempts: {} / {}", node_clone.attempts, node_clone.max_attempts));
                                            if let Some(ref refl) = node_clone.last_reflection {
                                                ui.label(format!("Reflection: {}", refl));
                                            }
                                        });

                                    node_response.context_menu(|ui| {
                                        ui.label(egui::RichText::new(format!("Task #{}", id_val.0)).strong());
                                        ui.separator();

                                        if ui.button(&replay_lbl).clicked() {
                                            let mut g = graph.clone();
                                            g.reset_task_and_dependents(id_val);
                                            graph_modified = Some(g);
                                            ui.close_menu();
                                        }
                                        if ui.button(&restart_lbl).clicked() {
                                            let mut g = graph.clone();
                                            g.reset_task(id_val);
                                            graph_modified = Some(g);
                                            ui.close_menu();
                                        }
                                        if ui.button(&complete_lbl).clicked() {
                                            let mut g = graph.clone();
                                            g.complete_task(id_val);
                                            graph_modified = Some(g);
                                            ui.close_menu();
                                        }
                                        if ui.button(&pending_lbl).clicked() {
                                            let mut g = graph.clone();
                                            g.reset_task(id_val);
                                            graph_modified = Some(g);
                                            ui.close_menu();
                                        }
                                    });
                                }
                            }
                        });
                }
            });

            ui.add_space(8.0);

            // --- Detailed Task List & Reflections (Bottom Section) ---
            ui.group(|ui| {
                ui.label(egui::RichText::new("📄 Detailed Tasks & Reflections").strong());
                ui.separator();
                egui::ScrollArea::vertical().id_salt("detailed_tasks_scroll").max_height(250.0).show(ui, |ui| {
                    let mut sorted_tasks: Vec<(&TaskId, &TaskNode)> = graph.tasks.iter().collect();
                    sorted_tasks.sort_by_key(|(id, _)| id.0);
                    for (id, node) in sorted_tasks {
                        let header_text = format!("#{} - {} - {:?}", id.0, node.description, node.status);
                        
                        ui.collapsing(header_text, |ui| {
                            ui.indent(format!("task_detail_{}", id.0), |ui| {
                                ui.horizontal(|ui| {
                                    let (status_color, status_text) = match &node.status {
                                        TaskStatus::Pending => (egui::Color32::from_rgb(100, 149, 237), "Pending".to_string()),
                                        TaskStatus::InProgress => (egui::Color32::from_rgb(255, 165, 0), "InProgress".to_string()),
                                        TaskStatus::Completed => (egui::Color32::from_rgb(50, 205, 50), "Completed".to_string()),
                                        TaskStatus::Failed { reason } => (egui::Color32::from_rgb(220, 20, 60), format!("Failed: {}", reason)),
                                        TaskStatus::Blocked => (egui::Color32::from_rgb(169, 169, 169), "Blocked".to_string()),
                                        TaskStatus::Skipped => (egui::Color32::from_rgb(120, 120, 200), "Skipped".to_string()),
                                    };
                                    ui.label("Status:");
                                    ui.colored_label(status_color, status_text);
                                    ui.add_space(12.0);
                                    ui.label(format!("Attempts: {}/{}", node.attempts, node.max_attempts));
                                });
                                
                                if let Some(ref reflection) = node.last_reflection {
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("Last Reflection:").strong());
                                    ui.colored_label(egui::Color32::LIGHT_GRAY, reflection);
                                }
                                
                                // Fetch associated steps with logs and images
                                let task_steps = get_steps_for_task(*id, &app.structured_steps);
                                if !task_steps.is_empty() {
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new("Execution Steps & Screenshots:").strong());
                                    for (step_idx, step) in task_steps {
                                        ui.group(|ui| {
                                            ui.label(egui::RichText::new(format!("Action: {}", step.action_type)).strong());
                                            if !step.tooltip.is_empty() {
                                                ui.label(format!("Info: {}", step.tooltip));
                                            }
                                            
                                            // Render step logs
                                            let step_logs = step.logs.join("\n");
                                            if !step_logs.is_empty() {
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(step_logs)
                                                        .monospace()
                                                        .color(egui::Color32::from_rgb(0, 220, 0))
                                                ).wrap());
                                            }
                                            
                                            // Render step screenshot/crop
                                            if let Some(ref bytes) = step.action_image {
                                                let texture = app.action_textures.entry(step_idx).or_insert_with(|| {
                                                    let img = image::load_from_memory(bytes).unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
                                                    let size = [img.width() as usize, img.height() as usize];
                                                    let pixels = img.to_rgba8();
                                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                                    ui.ctx().load_texture(
                                                        format!("task_step_crop_{}", step_idx),
                                                        color_image,
                                                        Default::default()
                                                    )
                                                });
                                                ui.add_space(4.0);
                                                ui.image(&*texture);
                                            }
                                        });
                                    }
                                }
                            });
                        });
                        ui.separator();
                    }
                });
            });

            if let Some(new_graph) = graph_modified {
                app.config_repo.save_task_graph(Some(new_graph));
                app.bus.emit_notification(NotificationEvent::Log("Task graph updated manually".to_string()));
            }
        } else {
            ui.colored_label(egui::Color32::YELLOW, "⚠️ No active TaskGraph exists. Generate prompts first or start orchestration to initialize.");
        }
    });
}

trait UiExtensions {
    fn subheader(&mut self, text: impl Into<String>);
}

impl UiExtensions for egui::Ui {
    fn subheader(&mut self, text: impl Into<String>) {
        self.label(egui::RichText::new(text.into()).heading().size(16.0));
        self.separator();
    }
}
