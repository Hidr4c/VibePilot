use eframe::egui;
use std::collections::{HashMap, HashSet};
use crate::app::VibePilotApp;
use crate::memory::{TaskId, TaskNode, TaskStatus};
use crate::event_bus::NotificationEvent;

pub fn render_task_graph_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.vertical(|ui| {
        ui.heading("🪵 Task Graph Visualizer & Editor");
        ui.label("Here you can inspect the current AI plan, visualize task dependencies, and manually modify the graph.");
        ui.separator();

        // Load the current task graph from the configuration repository
        let mut graph_opt = app.config_repo.load_task_graph();
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Reload Graph").clicked() {
                // Just reloading is enough as we read on each frame
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

            // Draw Editor and Visualizer side-by-side or stacked
            ui.columns(2, |columns| {
                // Left Column: Visualizer
                let ui_vis = &mut columns[0];
                ui_vis.vertical(|ui| {
                    ui.subheader("Visualizer");

                    if graph.tasks.is_empty() {
                        let size = egui::vec2(ui.available_width(), 400.0);
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
                                // Cycle fallback
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
                            .max_height(400.0)
                            .show(ui, |ui| {
                                let (response, painter) = ui.allocate_painter(size, egui::Sense::click() | egui::Sense::hover());
                                let rect = response.rect;
                                painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

                                // Compute node coordinates (vertical layout)
                                let padding_x = 50.0;
                                let mut coords: HashMap<TaskId, egui::Pos2> = HashMap::new();
                                for (layer, ids) in &layer_groups {
                                    let y = rect.top() + 30.0 + (*layer as f32 * layer_height);
                                    for (index, id) in ids.iter().enumerate() {
                                        // Index determines X
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

                                        // Draw circle
                                        painter.circle_filled(*pos, 16.0, color);
                                        painter.circle_stroke(*pos, 16.0, egui::Stroke::new(2.0, egui::Color32::WHITE));

                                        // Draw label text inside node
                                        painter.text(
                                            *pos,
                                            egui::Align2::CENTER_CENTER,
                                            label_prefix,
                                            egui::FontId::monospace(14.0),
                                            egui::Color32::WHITE,
                                        );

                                        // Draw task ID index and description next to node
                                        painter.text(
                                            *pos + egui::vec2(24.0, 0.0),
                                            egui::Align2::LEFT_CENTER,
                                            format!("#{} - {}", id.0, node.description),
                                            egui::FontId::proportional(12.0),
                                            ui.visuals().text_color(),
                                        );

                                        // Interactive rect for tooltip and right-click context menu
                                        let node_rect = egui::Rect::from_center_size(*pos, egui::vec2(32.0, 32.0));
                                        let node_id = ui.make_persistent_id(format!("task_node_{}", id.0));
                                        let node_response = ui.interact(node_rect, node_id, egui::Sense::click() | egui::Sense::hover());

                                        let node_clone = node.clone();
                                        let id_val = *id;

                                        // Tooltip
                                        let node_response = node_response.on_hover_ui(|ui| {
                                            ui.heading(format!("Task #{} details", id_val.0));
                                            ui.label(format!("Description: {}", node_clone.description));
                                            ui.label(format!("Status: {:?}", node_clone.status));
                                            ui.label(format!("Attempts: {} / {}", node_clone.attempts, node_clone.max_attempts));
                                            if let Some(ref refl) = node_clone.last_reflection {
                                                ui.label(format!("Reflection: {}", refl));
                                            }
                                        });

                                        // Right-click context menu
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

                // Right Column: Editor
                let ui_edit = &mut columns[1];
                ui_edit.vertical(|ui| {
                    ui.subheader("Editor Tools");

                    // 1. Add Task Form
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("➕ Add New Task").strong());
                        
                        // We use static state fields on VibePilotApp to keep input values
                        ui.horizontal(|ui| {
                            ui.label("Description:");
                            ui.text_edit_singleline(&mut app.user_feedback_input); // Repurposed for simplicity or using state fields
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

                    ui.add_space(8.0);

                    // 2. Add Dependency Form
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🔗 Add Dependency (Child -> Parent)").strong());

                        // Simple text boxes to type ID

                        ui.horizontal(|ui| {
                            ui.label("Child ID:");
                            ui.add(egui::TextEdit::singleline(&mut app.selected_profile_to_export).desired_width(50.0)); // Repurposed
                            ui.label("depends on Parent ID:");
                            ui.add(egui::TextEdit::singleline(&mut app.selected_engine_to_export).desired_width(50.0)); // Repurposed
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
                                    // Check for cycles
                                    if graph.has_cycle() {
                                        app.bus.emit_notification(NotificationEvent::Log("⚠️ Cannot add dependency: cycle detected!".to_string()));
                                        // Rollback
                                        if let Some(node) = graph.tasks.get_mut(&child) {
                                            node.dependencies.retain(|&x| x != parent);
                                        }
                                    } else {
                                        app.config_repo.save_task_graph(Some(graph.clone()));
                                        app.bus.emit_notification(NotificationEvent::Log(format!("Connected Task #{} to #{}", child.0, parent.0)));
                                    }
                                } else {
                                    app.bus.emit_notification(NotificationEvent::Log("⚠️ One or both Task IDs do not exist".to_string()));
                                }
                            }
                        }
                    });

                    ui.add_space(8.0);

                    // 3. Delete Task or Dependency List
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🗑️ Manage Tasks").strong());
                        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                            let tasks_info: Vec<(TaskId, String)> = graph.tasks.iter()
                                .map(|(&id, node)| (id, node.description.clone()))
                                .collect();
                            for (id, desc) in tasks_info {
                                ui.horizontal(|ui| {
                                    ui.label(format!("#{} - {}", id.0, &desc[..desc.len().min(20)]));
                                    
                                    if ui.button("Delete").clicked() {
                                        graph.tasks.remove(&id);
                                        graph.root_order.retain(|&x| x != id);
                                        // Remove from other dependencies
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
            if let Some(new_graph) = graph_modified {
                app.config_repo.save_task_graph(Some(new_graph));
                app.bus.emit_notification(NotificationEvent::Log("Task graph updated manually".to_string()));
            }
        } else {
            ui.colored_label(egui::Color32::YELLOW, "⚠️ No active TaskGraph exists. Generate prompts first or start orchestration to initialize.");
        }
    });
}

// Helper trait to allow ui subheaders
trait UiExtensions {
    fn subheader(&mut self, text: impl Into<String>);
}

impl UiExtensions for egui::Ui {
    fn subheader(&mut self, text: impl Into<String>) {
        self.label(egui::RichText::new(text.into()).heading().size(16.0));
        self.separator();
    }
}
