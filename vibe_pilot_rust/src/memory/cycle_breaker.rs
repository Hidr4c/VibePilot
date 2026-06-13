use crate::memory::{TaskGraph, TaskId};

pub fn break_cycles_auto(graph: &mut TaskGraph) -> Result<(), String> {
    if !graph.has_cycle() {
        return Ok(());
    }

    // Try each edge removal to find one that breaks the cycle
    let task_ids: Vec<TaskId> = graph.tasks.keys().copied().collect();
    let mut removed = None;

    for child_id in &task_ids {
        if let Some(task) = graph.tasks.get(child_id) {
            let deps: Vec<TaskId> = task.dependencies.clone();
            for dep_id in deps {
                // Temporarily remove the dependency
                if let Some(task_mut) = graph.tasks.get_mut(child_id) {
                    task_mut.dependencies.retain(|d| d != &dep_id);
                }
                if !graph.has_cycle() {
                    removed = Some((*child_id, dep_id));
                    // Restore dependency was already removed — keep it removed
                    break;
                }
                // Restore dependency
                if let Some(task_mut) = graph.tasks.get_mut(child_id) {
                    task_mut.dependencies.push(dep_id);
                }
            }
        }
        if removed.is_some() {
            break;
        }
    }

    match removed {
        Some((child, dep)) => {
            log::warn!(
                "TaskGraph cycle detected and auto-fixed: removed dependency {} -> {}",
                child.0,
                dep.0
            );
            Ok(())
        }
        None => Err("TaskGraph: could not resolve circular dependencies".to_string()),
    }
}

pub fn break_cycles(graph: &mut TaskGraph) -> Option<(TaskId, TaskId)> {
    if !graph.has_cycle() {
        return None;
    }

    // Find all edges (child -> parent dependency) and try removing each
    let task_ids: Vec<TaskId> = graph.tasks.keys().copied().collect();
    for child_id in &task_ids {
        if let Some(task) = graph.tasks.get(child_id) {
            let deps: Vec<TaskId> = task.dependencies.clone();
            for dep_id in deps {
                // Remove dependency temporarily
                if let Some(task_mut) = graph.tasks.get_mut(child_id) {
                    task_mut.dependencies.retain(|d| d != &dep_id);
                }
                // Check if cycle is resolved
                if !graph.has_cycle() {
                    return Some((*child_id, dep_id));
                }
                // Restore dependency
                if let Some(task_mut) = graph.tasks.get_mut(child_id) {
                    task_mut.dependencies.push(dep_id);
                }
            }
        }
    }
    None
}
