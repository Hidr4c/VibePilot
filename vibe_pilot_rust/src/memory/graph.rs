use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::node::{TaskId, TaskStatus, TaskNode};

/// Directed Acyclic Graph of sub-tasks decomposed from a user objective.
///
/// The orchestrator uses this graph to:
/// 1. Know which sub-task to work on next (`next_ready_task`)
/// 2. Track progress across the whole objective
/// 3. Inject a compact plan summary into each LLM prompt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskGraph {
    /// All task nodes indexed by their ID.
    pub tasks: HashMap<TaskId, TaskNode>,
    /// Root-level task IDs (tasks with no parent).
    pub root_order: Vec<TaskId>,
    /// ID of the task currently being worked on.
    pub current_task_id: Option<TaskId>,
    /// Counter for generating unique IDs.
    pub next_id: u64,
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskGraph {
    /// Creates an empty task graph.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            root_order: Vec::new(),
            current_task_id: None,
            next_id: 1,
        }
    }

    /// Parses a JSON response from the LLM planning call and builds the graph.
    ///
    /// Expected JSON format:
    /// ```json
    /// [
    ///   {"id": 1, "description": "Open Chrome", "depends_on": []},
    ///   {"id": 2, "description": "Navigate to Gmail", "depends_on": [1]}
    /// ]
    /// ```
    ///
    /// Returns `Ok(())` on success, or an error message if parsing fails.
    pub fn from_llm_response(json_text: &str) -> Result<Self, String> {
        // Strip markdown code fences if present
        let cleaned = json_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        #[derive(Deserialize)]
        struct RawTask {
            id: u64,
            description: String,
            #[serde(default)]
            depends_on: Vec<u64>,
        }

        let raw_tasks: Vec<RawTask> = serde_json::from_str(cleaned)
            .map_err(|e| format!("Failed to parse task graph JSON: {}", e))?;

        if raw_tasks.is_empty() {
            return Err("LLM returned empty task list".to_string());
        }

        let mut graph = Self::new();
        let mut order = Vec::new();

        for raw in &raw_tasks {
            let id = TaskId(raw.id);
            let deps: Vec<TaskId> = raw.depends_on.iter().map(|d| TaskId(*d)).collect();
            let node = TaskNode::new(id, raw.description.clone(), deps);
            graph.tasks.insert(id, node);
            order.push(id);
            if raw.id >= graph.next_id {
                graph.next_id = raw.id + 1;
            }
        }

        graph.root_order = order;

        // Detect and auto-fix cycles before returning the graph
        if graph.has_cycle() {
            log::warn!("TaskGraph: circular dependencies detected in LLM response, attempting auto-fix");
            if let Err(e) = graph.break_cycles_auto() {
                return Err(format!("LLM returned a task graph with circular dependencies that could not be auto-fixed: {}", e));
            }
            log::info!("TaskGraph: cycles auto-fixed successfully");
        }

        Ok(graph)
    }

    /// Adds a task to the graph.
    pub fn add_task(&mut self, task: TaskNode) {
        let id = task.id;
        if id.0 >= self.next_id {
            self.next_id = id.0 + 1;
        }
        if !self.root_order.contains(&id) {
            self.root_order.push(id);
        }
        self.tasks.insert(id, task);
    }

    /// Sets the maximum attempts for all tasks in the graph.
    pub fn set_max_attempts(&mut self, max: u32) {
        for task in self.tasks.values_mut() {
            task.max_attempts = max;
        }
    }

    /// Returns the number of tasks in the graph.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` if the graph has no tasks.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Returns the task node for the given ID if it exists.
    pub fn get_task(&self, id: TaskId) -> Option<&TaskNode> {
        self.tasks.get(&id)
    }

    /// Returns the next task that is ready to execute.
    ///
    /// A task is "ready" if:
    /// - Its status is `Pending`
    /// - All its dependencies are `Completed`
    ///
    /// Returns tasks in insertion order (respecting `root_order`).
    pub fn next_ready_task(&self) -> Option<&TaskNode> {
        for id in &self.root_order {
            if let Some(task) = self.tasks.get(id) {
                if task.status == TaskStatus::Pending && self.deps_satisfied(id) {
                    return Some(task);
                }
            }
        }
        None
    }

    /// Checks if all dependencies of a task are completed.
    fn deps_satisfied(&self, task_id: &TaskId) -> bool {
        if let Some(task) = self.tasks.get(task_id) {
            task.dependencies.iter().all(|dep_id| {
                self.tasks
                    .get(dep_id)
                    .map(|dep| dep.status == TaskStatus::Completed)
                    .unwrap_or(false)
            })
        } else {
            false
        }
    }

    /// Marks a task as InProgress and sets it as the current task.
    pub fn start_task(&mut self, task_id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::InProgress;
            task.attempts += 1;
        }
        self.current_task_id = Some(task_id);
    }

    /// Marks a task as Completed.
    pub fn complete_task(&mut self, task_id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
        }
        if self.current_task_id == Some(task_id) {
            self.current_task_id = None;
        }
        self.update_blocked_states();
    }

    /// Marks a task as Failed (with reason) and blocks any dependent tasks.
    pub fn fail_task(&mut self, task_id: TaskId, reason: String) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Failed { reason };
        }
        if self.current_task_id == Some(task_id) {
            self.current_task_id = None;
        }
        self.update_blocked_states();
    }

    /// Resets a single task to Pending state, clearing its attempts and reflection.
    pub fn reset_task(&mut self, id: TaskId) {
        self.reset_task_internal(id);
        self.update_blocked_states();
    }

    /// Resets a task and all other tasks that transitively depend on it back to Pending.
    pub fn reset_task_and_dependents(&mut self, start_id: TaskId) {
        self.reset_task_internal(start_id);
        
        let dependents: Vec<TaskId> = self.tasks.iter()
            .filter(|(_, t)| t.dependencies.contains(&start_id))
            .map(|(id, _)| *id)
            .collect();
            
        for dep_id in dependents {
            self.reset_task_and_dependents_internal(dep_id);
        }
        
        self.update_blocked_states();
    }

    fn reset_task_internal(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = TaskStatus::Pending;
            task.attempts = 0;
            task.last_reflection = None;
        }
    }

    fn reset_task_and_dependents_internal(&mut self, start_id: TaskId) {
        self.reset_task_internal(start_id);
        
        let dependents: Vec<TaskId> = self.tasks.iter()
            .filter(|(_, t)| t.dependencies.contains(&start_id))
            .map(|(id, _)| *id)
            .collect();
            
        for dep_id in dependents {
            self.reset_task_and_dependents_internal(dep_id);
        }
    }

    /// Recalculates the status of all tasks in the graph to ensure Blocked states are consistent.
    /// A task is Blocked if any of its dependencies is Failed or Blocked.
    /// If a task was Blocked but all its dependencies are now satisfied or Pending, it returns to Pending.
    pub fn update_blocked_states(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            let task_ids: Vec<TaskId> = self.tasks.keys().copied().collect();
            for id in task_ids {
                let node = &self.tasks[&id];
                let should_be_blocked = node.dependencies.iter().any(|dep_id| {
                    self.tasks.get(dep_id)
                        .map(|dep| matches!(dep.status, TaskStatus::Failed { .. } | TaskStatus::Blocked))
                        .unwrap_or(false)
                });

                let current_blocked = matches!(node.status, TaskStatus::Blocked);
                if should_be_blocked && !current_blocked {
                    if let Some(n) = self.tasks.get_mut(&id) {
                        n.status = TaskStatus::Blocked;
                        changed = true;
                    }
                } else if !should_be_blocked && current_blocked {
                    if let Some(n) = self.tasks.get_mut(&id) {
                        n.status = TaskStatus::Pending;
                        changed = true;
                    }
                }
            }
        }
    }

    /// Marks a task as Failed if it has exceeded max_attempts.
    /// Returns `true` if the task was auto-failed.
    pub fn check_and_fail_exhausted(&mut self, task_id: TaskId) -> bool {
        let should_fail = self.tasks.get(&task_id)
            .map(|t| t.attempts >= t.max_attempts && !t.is_terminal())
            .unwrap_or(false);

        if should_fail {
            self.fail_task(task_id, format!("Exhausted {} attempts", 
                self.tasks.get(&task_id).map(|t| t.max_attempts).unwrap_or(0)));
            true
        } else {
            false
        }
    }

    /// Sets a reflection note on a task.
    pub fn set_reflection(&mut self, task_id: TaskId, reflection: String) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.last_reflection = Some(reflection);
        }
    }

    /// Returns `true` if all tasks are in a terminal state.
    pub fn is_complete(&self) -> bool {
        self.tasks.values().all(|t| t.is_terminal())
    }

    /// Returns the currently active task ID.
    pub fn current_task(&self) -> Option<TaskId> {
        self.current_task_id
    }

    /// Returns progress as (completed, total).
    pub fn progress(&self) -> (usize, usize) {
        let completed = self.tasks.values()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        (completed, self.tasks.len())
    }

    /// Detects if the task graph contains any circular dependencies.
    ///
    /// Uses a depth-first search with a recursion stack to identify cycles.
    /// Returns `true` if a cycle is detected, `false` otherwise.
    pub fn has_cycle(&self) -> bool {
        let task_ids: Vec<TaskId> = self.tasks.keys().copied().collect();
        let n = task_ids.len();
        if n == 0 {
            return false;
        }

        let mut visited = vec![false; n];
        let mut rec_stack = vec![false; n];

        for (i, _) in task_ids.iter().enumerate() {
            if !visited[i]
                && self.detect_cycle_dfs(i, &task_ids, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn detect_cycle_dfs(
        &self,
        idx: usize,
        task_ids: &[TaskId],
        visited: &mut [bool],
        rec_stack: &mut [bool],
    ) -> bool {
        rec_stack[idx] = true;
        visited[idx] = true;

        if let Some(task) = self.tasks.get(&task_ids[idx]) {
            for dep_id in &task.dependencies {
                if let Some(dep_idx) = task_ids.iter().position(|id| id == dep_id) {
                    if !visited[dep_idx] {
                        if self.detect_cycle_dfs(dep_idx, task_ids, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack[dep_idx] {
                        return true;
                    }
                }
            }
        }

        rec_stack[idx] = false;
        false
    }

    /// Detects and breaks cycles in the task graph.
    ///
    /// Returns `Ok(())` if the graph is acyclic. If a cycle exists,
    /// attempts to break it by removing the dependency on the task
    /// with the highest `max_attempts` (least critical), then returns `Ok`
    /// with a log message describing the fix.
    pub fn break_cycles_auto(&mut self) -> Result<(), String> {
        super::cycle_breaker::break_cycles_auto(self)
    }

    /// Attempts to break cycles in the task graph by removing the dependency
    /// that creates the shortest cycle path.
    ///
    /// Returns `Some(removed_dependency)` if a cycle was broken, `None` if no cycles existed.
    pub fn break_cycles(&mut self) -> Option<(TaskId, TaskId)> {
        super::cycle_breaker::break_cycles(self)
    }


    /// Formats the task graph as a compact text block for injection into the LLM prompt.
    ///
    /// # Arguments
    /// * `lang` - `"Français"` or `"English"` for localized labels.
    pub fn format_for_prompt(&self, lang: &str) -> String {
        if self.tasks.is_empty() {
            return String::new();
        }

        let (completed, total) = self.progress();
        let is_fr = lang.contains("Fran");

        let title = if is_fr {
            format!("\n\n🗺️ PLAN DE TÂCHES ({}/{} terminées):", completed, total)
        } else {
            format!("\n\n🗺️ TASK PLAN ({}/{} completed):", completed, total)
        };

        let mut lines = vec![title];

        for id in &self.root_order {
            if let Some(task) = self.tasks.get(id) {
                let icon = match &task.status {
                    TaskStatus::Completed => "[✅]",
                    TaskStatus::InProgress => "[🔄]",
                    TaskStatus::Failed { .. } => "[❌]",
                    TaskStatus::Blocked => "[⛔]",
                    TaskStatus::Skipped => "[⏭️]",
                    TaskStatus::Pending => "[ ]",
                };

                let mut line = format!("{} {}. {}", icon, id.0, task.description);

                if task.status == TaskStatus::InProgress {
                    let current_marker = if is_fr { "← EN COURS" } else { "← CURRENT" };
                    line = format!("{} {} (tentative {}/{})", line, current_marker, task.attempts, task.max_attempts);
                }

                if let TaskStatus::Failed { ref reason } = task.status {
                    line = format!("{} ({})", line, reason);
                }

                if let Some(ref reflection) = task.last_reflection {
                    let preview = if reflection.len() > 60 {
                        format!("{}...", &reflection[..57])
                    } else {
                        reflection.clone()
                    };
                    line = format!("{} [💭 {}]", line, preview);
                }

                lines.push(line);
            }
        }

        lines.join("\n")
    }
}
