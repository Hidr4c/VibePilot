use serde::{Serialize, Deserialize};

/// Unique identifier for a task node in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub u64);

/// Status of a task node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task has not been started yet.
    Pending,
    /// Task is currently being worked on.
    InProgress,
    /// Task completed successfully.
    Completed,
    /// Task failed after exhausting retries.
    Failed { reason: String },
    /// Task is blocked because a dependency failed or is blocked.
    Blocked,
    /// Task was intentionally skipped (e.g., not relevant anymore).
    Skipped,
}

/// A single task node in the DAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskNode {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable description of the sub-task.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// Parent task ID (for hierarchical decomposition).
    pub parent: Option<TaskId>,
    /// IDs of tasks that must be `Completed` before this one can start.
    pub dependencies: Vec<TaskId>,
    /// Number of execution attempts so far.
    pub attempts: u32,
    /// Maximum attempts before marking as Failed.
    pub max_attempts: u32,
    /// Last reflection/evaluation note from the ReflectionModule.
    pub last_reflection: Option<String>,
}

impl TaskNode {
    /// Creates a new task node with default retry settings.
    pub fn new(id: TaskId, description: String, dependencies: Vec<TaskId>) -> Self {
        Self {
            id,
            description,
            status: TaskStatus::Pending,
            parent: None,
            dependencies,
            attempts: 0,
            max_attempts: 5,
            last_reflection: None,
        }
    }

    /// Returns `true` if this task can be considered "done" (completed, failed, or skipped).
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Skipped)
    }
}
