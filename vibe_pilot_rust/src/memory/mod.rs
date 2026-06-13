//! Hierarchical task memory and structured task graph (DAG).
//!
//! Provides a graph-based decomposition of user objectives into ordered sub-tasks,
//! plus a two-level memory system (short-term session / long-term patterns).

pub mod node;
pub mod graph;
pub mod cycle_breaker;

#[cfg(test)]
mod tests;

pub use node::{TaskId, TaskStatus, TaskNode};
pub use graph::TaskGraph;
