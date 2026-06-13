use super::node::{TaskId, TaskStatus, TaskNode};
use super::graph::TaskGraph;

#[test]
fn test_task_id_equality() {
    assert_eq!(TaskId(1), TaskId(1));
    assert_ne!(TaskId(1), TaskId(2));
}

#[test]
fn test_task_node_new() {
    let node = TaskNode::new(TaskId(1), "Test task".into(), vec![]);
    assert_eq!(node.id, TaskId(1));
    assert_eq!(node.status, TaskStatus::Pending);
    assert_eq!(node.attempts, 0);
    assert_eq!(node.max_attempts, 5);
    assert!(node.dependencies.is_empty());
    assert!(!node.is_terminal());
}

#[test]
fn test_task_node_is_terminal() {
    let mut node = TaskNode::new(TaskId(1), "Test".into(), vec![]);
    assert!(!node.is_terminal());

    node.status = TaskStatus::Completed;
    assert!(node.is_terminal());

    node.status = TaskStatus::Failed { reason: "test".into() };
    assert!(node.is_terminal());

    node.status = TaskStatus::Skipped;
    assert!(node.is_terminal());

    node.status = TaskStatus::InProgress;
    assert!(!node.is_terminal());

    node.status = TaskStatus::Blocked;
    assert!(!node.is_terminal());
}

#[test]
fn test_empty_graph() {
    let graph = TaskGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert!(graph.is_complete());
    assert!(graph.next_ready_task().is_none());
}

#[test]
fn test_add_and_traverse() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Step 1".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Step 2".into(), vec![TaskId(1)]));
    graph.add_task(TaskNode::new(TaskId(3), "Step 3".into(), vec![TaskId(2)]));

    assert_eq!(graph.len(), 3);
    assert!(!graph.is_complete());

    // Only task 1 is ready (no deps)
    let next = graph.next_ready_task().unwrap();
    assert_eq!(next.id, TaskId(1));

    // Start and complete task 1
    graph.start_task(TaskId(1));
    assert_eq!(graph.current_task(), Some(TaskId(1)));
    graph.complete_task(TaskId(1));

    // Now task 2 is ready
    let next = graph.next_ready_task().unwrap();
    assert_eq!(next.id, TaskId(2));
}

#[test]
fn test_set_max_attempts() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Step 1".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Step 2".into(), vec![]));
    
    graph.set_max_attempts(8);
    assert_eq!(graph.tasks[&TaskId(1)].max_attempts, 8);
    assert_eq!(graph.tasks[&TaskId(2)].max_attempts, 8);
}


#[test]
fn test_dependency_blocking() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Step 1".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Step 2".into(), vec![TaskId(1)]));
    graph.add_task(TaskNode::new(TaskId(3), "Step 3".into(), vec![TaskId(2)]));

    // Fail task 1 → task 2 and 3 should be blocked
    graph.fail_task(TaskId(1), "Could not click".into());

    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Blocked));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Blocked));
}

#[test]
fn test_progress() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(3), "C".into(), vec![]));

    assert_eq!(graph.progress(), (0, 3));

    graph.complete_task(TaskId(1));
    assert_eq!(graph.progress(), (1, 3));

    graph.complete_task(TaskId(2));
    graph.complete_task(TaskId(3));
    assert_eq!(graph.progress(), (3, 3));
    assert!(graph.is_complete());
}

#[test]
fn test_check_and_fail_exhausted() {
    let mut graph = TaskGraph::new();
    let mut node = TaskNode::new(TaskId(1), "Retry me".into(), vec![]);
    node.max_attempts = 3;
    graph.add_task(node);

    graph.start_task(TaskId(1)); // attempt 1
    assert!(!graph.check_and_fail_exhausted(TaskId(1)));

    // Simulate retries: reset to Pending and start again
    graph.tasks.get_mut(&TaskId(1)).unwrap().status = TaskStatus::Pending;
    graph.start_task(TaskId(1)); // attempt 2
    assert!(!graph.check_and_fail_exhausted(TaskId(1)));

    graph.tasks.get_mut(&TaskId(1)).unwrap().status = TaskStatus::Pending;
    graph.start_task(TaskId(1)); // attempt 3 = max
    assert!(graph.check_and_fail_exhausted(TaskId(1)));
    assert!(matches!(graph.tasks[&TaskId(1)].status, TaskStatus::Failed { .. }));
}

#[test]
fn test_from_llm_response() {
    let json = r#"[
        {"id": 1, "description": "Open Chrome", "depends_on": []},
        {"id": 2, "description": "Navigate to Gmail", "depends_on": [1]},
        {"id": 3, "description": "Click Compose", "depends_on": [2]}
    ]"#;

    let graph = TaskGraph::from_llm_response(json).unwrap();
    assert_eq!(graph.len(), 3);
    assert_eq!(graph.next_ready_task().unwrap().id, TaskId(1));
    assert_eq!(graph.tasks[&TaskId(2)].dependencies, vec![TaskId(1)]);
}

#[test]
fn test_from_llm_response_with_markdown_fences() {
    let json = "```json\n[\n{\"id\": 1, \"description\": \"Step 1\", \"depends_on\": []}\n]\n```";
    let graph = TaskGraph::from_llm_response(json).unwrap();
    assert_eq!(graph.len(), 1);
}

#[test]
fn test_from_llm_response_invalid() {
    let result = TaskGraph::from_llm_response("not json at all");
    assert!(result.is_err());
}

#[test]
fn test_from_llm_response_empty() {
    let result = TaskGraph::from_llm_response("[]");
    assert!(result.is_err());
}

#[test]
fn test_format_for_prompt_english() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Open Chrome".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Navigate to Gmail".into(), vec![TaskId(1)]));

    graph.complete_task(TaskId(1));
    graph.start_task(TaskId(2));

    let prompt = graph.format_for_prompt("English");
    assert!(prompt.contains("TASK PLAN"));
    assert!(prompt.contains("1/2 completed"));
    assert!(prompt.contains("[✅] 1. Open Chrome"));
    assert!(prompt.contains("[🔄] 2. Navigate to Gmail"));
    assert!(prompt.contains("CURRENT"));
}

#[test]
fn test_format_for_prompt_french() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Ouvrir Chrome".into(), vec![]));
    graph.fail_task(TaskId(1), "Fenêtre introuvable".into());

    let prompt = graph.format_for_prompt("Français");
    assert!(prompt.contains("PLAN DE TÂCHES"));
    assert!(prompt.contains("[❌]"));
    assert!(prompt.contains("Fenêtre introuvable"));
}

#[test]
fn test_format_for_prompt_empty_graph() {
    let graph = TaskGraph::new();
    assert!(graph.format_for_prompt("English").is_empty());
}

#[test]
fn test_set_reflection() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Click button".into(), vec![]));
    graph.set_reflection(TaskId(1), "Button was not visible, need to scroll".into());

    let prompt = graph.format_for_prompt("English");
    assert!(prompt.contains("💭"));
    assert!(prompt.contains("Button was not visible"));
}

#[test]
fn test_parallel_tasks() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Task A".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Task B".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(3), "Final".into(), vec![TaskId(1), TaskId(2)]));

    // Both task 1 and 2 are ready (no deps)
    let next = graph.next_ready_task().unwrap();
    assert_eq!(next.id, TaskId(1)); // First in insertion order

    graph.complete_task(TaskId(1));
    let next = graph.next_ready_task().unwrap();
    assert_eq!(next.id, TaskId(2)); // Task 2 is next

    // Task 3 not ready yet (task 2 not complete)
    graph.complete_task(TaskId(2));
    let next = graph.next_ready_task().unwrap();
    assert_eq!(next.id, TaskId(3)); // Now task 3 is ready
}

#[test]
fn test_skip_status() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Optional step".into(), vec![]));

    graph.tasks.get_mut(&TaskId(1)).unwrap().status = TaskStatus::Skipped;
    assert!(graph.tasks[&TaskId(1)].is_terminal());
    assert!(graph.is_complete());
}

#[test]
fn test_has_cycle_no_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Step 1".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Step 2".into(), vec![TaskId(1)]));
    graph.add_task(TaskNode::new(TaskId(3), "Step 3".into(), vec![TaskId(2)]));
    assert!(!graph.has_cycle());
}

#[test]
fn test_has_cycle_direct_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![TaskId(2)]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    assert!(graph.has_cycle());
}

#[test]
fn test_has_cycle_indirect_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    graph.add_task(TaskNode::new(TaskId(3), "C".into(), vec![TaskId(2)]));
    // Manually create cycle: C depends on B, B depends on A, but we make A depend on C
    graph.tasks.get_mut(&TaskId(1)).unwrap().dependencies.push(TaskId(3));
    assert!(graph.has_cycle());
}

#[test]
fn test_has_cycle_empty_graph() {
    let graph = TaskGraph::new();
    assert!(!graph.has_cycle());
}

#[test]
fn test_has_cycle_single_node_no_deps() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Solo".into(), vec![]));
    assert!(!graph.has_cycle());
}

#[test]
fn test_break_cycles_returns_some() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![TaskId(2)]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    assert!(graph.has_cycle());
    let result = graph.break_cycles();
    assert!(result.is_some());
    assert!(!graph.has_cycle());
}

#[test]
fn test_break_cycles_returns_none_when_no_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    assert!(!graph.has_cycle());
    let result = graph.break_cycles();
    assert!(result.is_none());
}

#[test]
fn test_from_llm_response_rejects_cycle() {
    // Create JSON where task 2 depends on task 3, and task 3 depends on task 2
    // but task 1 has no deps — this should now auto-fix by removing one edge
    let json = r#"[
        {"id": 1, "description": "Step 1", "depends_on": [3]},
        {"id": 2, "description": "Step 2", "depends_on": [1]},
        {"id": 3, "description": "Step 3", "depends_on": [2]}
    ]"#;
    let result = TaskGraph::from_llm_response(json);
    // With auto-break, this should now succeed
    assert!(result.is_ok(), "Expected auto-fix to resolve the cycle, got: {:?}", result.err());
}

#[test]
fn test_from_llm_response_unresolvable_cycle() {
    // Self-referential task (depends on itself) — the auto-break will remove that single edge,
    // making it resolvable. This test verifies the new auto-break behavior.
    let json = r#"[
        {"id": 1, "description": "Step 1", "depends_on": [1]}
    ]"#;
    let result = TaskGraph::from_llm_response(json);
    // With auto-break, self-reference is removed and graph becomes acyclic
    assert!(result.is_ok(), "Expected self-reference to be auto-fixed, got: {:?}", result.err());
}

#[test]
fn test_break_cycles_auto_resolves_simple_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![TaskId(2)]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    assert!(graph.has_cycle());
    assert!(graph.break_cycles_auto().is_ok());
    assert!(!graph.has_cycle());
}

#[test]
fn test_break_cycles_auto_no_cycle() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "A".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "B".into(), vec![TaskId(1)]));
    assert!(!graph.has_cycle());
    assert!(graph.break_cycles_auto().is_ok());
}

#[test]
fn test_task_graph_reset_and_blocking_propagation() {
    let mut graph = TaskGraph::new();
    graph.add_task(TaskNode::new(TaskId(1), "Step 1".into(), vec![]));
    graph.add_task(TaskNode::new(TaskId(2), "Step 2".into(), vec![TaskId(1)]));
    graph.add_task(TaskNode::new(TaskId(3), "Step 3".into(), vec![TaskId(2)]));

    // Fail task 1
    graph.fail_task(TaskId(1), "Failed first step".into());
    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Blocked));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Blocked));

    // Reset task 1
    graph.reset_task(TaskId(1));
    assert!(matches!(graph.tasks[&TaskId(1)].status, TaskStatus::Pending));
    // Resetting task 1 should unblock task 2 and 3 back to Pending
    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Pending));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Pending));

    // Fail task 1 again
    graph.fail_task(TaskId(1), "Failed first step again".into());
    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Blocked));

    // Complete task 1 manually (using complete_task)
    graph.complete_task(TaskId(1));
    // Completing task 1 should unblock task 2 and 3
    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Pending));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Pending));

    // Complete task 2
    graph.complete_task(TaskId(2));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Pending));

    // Fail task 2
    graph.fail_task(TaskId(2), "Oops".into());
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Blocked));

    // Reset task 2 and dependents
    graph.reset_task_and_dependents(TaskId(2));
    assert!(matches!(graph.tasks[&TaskId(2)].status, TaskStatus::Pending));
    assert!(matches!(graph.tasks[&TaskId(3)].status, TaskStatus::Pending));
}
