use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Execution state of a worker node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Pending,
    Running,
    Success,
    Failed,
}

/// A worker task node in the grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridNode {
    pub id: String,
    pub name: String,
    pub profile_name: String,
    pub worker_url: String,
    pub dependencies: Vec<String>,
    pub state: NodeState,
    pub start_time_offset: f32, // for Gantt visualization
    pub duration_secs: f32,      // simulated/actual execution duration
}

impl GridNode {
    pub fn new(id: String, name: String, profile_name: String, worker_url: String) -> Self {
        Self {
            id,
            name,
            profile_name,
            worker_url,
            dependencies: Vec::new(),
            state: NodeState::Pending,
            start_time_offset: 0.0,
            duration_secs: 10.0,
        }
    }
}

/// Directed Acyclic Graph (DAG) for orchestrating worker nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GridGraph {
    pub nodes: HashMap<String, GridNode>,
}

impl GridGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: GridNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let Some(node) = self.nodes.get_mut(to) {
            if !node.dependencies.contains(&from.to_string()) {
                node.dependencies.push(from.to_string());
            }
        }
    }

    /// Check if adding a dependency would introduce a cycle.
    pub fn would_cycle(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }
        let mut visited = HashSet::new();
        let mut stack = vec![from.to_string()];
        
        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }
            if visited.insert(current.clone()) {
                if let Some(node) = self.nodes.get(&current) {
                    for dep in &node.dependencies {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        false
    }

    /// Computes the Gantt chart schedule start times based on dependencies.
    pub fn compute_schedule(&mut self) {
        let mut resolved = HashMap::new();
        let keys: Vec<String> = self.nodes.keys().cloned().collect();

        for id in &keys {
            self.resolve_node_offset(id, &mut resolved);
        }

        for (id, offset) in resolved {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.start_time_offset = offset;
            }
        }
    }

    fn resolve_node_offset(&self, id: &str, resolved: &mut HashMap<String, f32>) -> f32 {
        if let Some(&offset) = resolved.get(id) {
            return offset;
        }

        let node = match self.nodes.get(id) {
            Some(n) => n,
            None => return 0.0,
        };

        let mut max_parent_end = 0.0f32;
        for dep in &node.dependencies {
            if let Some(parent) = self.nodes.get(dep) {
                let parent_start = self.resolve_node_offset(dep, resolved);
                let parent_end = parent_start + parent.duration_secs;
                if parent_end > max_parent_end {
                    max_parent_end = parent_end;
                }
            }
        }

        resolved.insert(id.to_string(), max_parent_end);
        
        // unsafe block or mutable get to set node offset (workaround for borrow checker)
        // Since we resolved offsets, we will write them in a separate step or via interior mutability.
        max_parent_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_node_new() {
        let node = GridNode::new(
            "node1".to_string(),
            "Node 1".to_string(),
            "profile".to_string(),
            "http://127.0.0.1".to_string(),
        );
        assert_eq!(node.id, "node1");
        assert_eq!(node.name, "Node 1");
        assert_eq!(node.profile_name, "profile");
        assert_eq!(node.worker_url, "http://127.0.0.1");
        assert_eq!(node.state, NodeState::Pending);
        assert_eq!(node.duration_secs, 10.0);
    }

    #[test]
    fn test_grid_graph_cycle_detection() {
        let mut graph = GridGraph::new();
        let n1 = GridNode::new("A".to_string(), "A".to_string(), "p".to_string(), "url".to_string());
        let n2 = GridNode::new("B".to_string(), "B".to_string(), "p".to_string(), "url".to_string());
        let n3 = GridNode::new("C".to_string(), "C".to_string(), "p".to_string(), "url".to_string());

        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);

        // Self cycle
        assert!(graph.would_cycle("A", "A"));

        // Direct cycle A -> B and B -> A
        graph.add_dependency("A", "B");
        assert!(graph.would_cycle("B", "A"));
        assert!(!graph.would_cycle("A", "B")); // dependency already exists, doesn't cycle

        // Indirect cycle A -> B -> C -> A
        graph.add_dependency("B", "C");
        assert!(graph.would_cycle("C", "A"));

        // No cycle
        assert!(!graph.would_cycle("B", "C"));
    }

    #[test]
    fn test_grid_graph_scheduling() {
        let mut graph = GridGraph::new();
        let mut n1 = GridNode::new("A".to_string(), "A".to_string(), "p".to_string(), "url".to_string());
        n1.duration_secs = 5.0;
        let mut n2 = GridNode::new("B".to_string(), "B".to_string(), "p".to_string(), "url".to_string());
        n2.duration_secs = 15.0;
        let mut n3 = GridNode::new("C".to_string(), "C".to_string(), "p".to_string(), "url".to_string());
        n3.duration_secs = 10.0;

        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);

        // A -> C and B -> C
        graph.add_dependency("A", "C");
        graph.add_dependency("B", "C");

        graph.compute_schedule();

        // Node A starts at 0, Node B starts at 0
        // Node C must start after both A and B are finished.
        // A ends at 5, B ends at 15. So C starts at 15.
        assert_eq!(graph.nodes.get("C").unwrap().start_time_offset, 15.0);
    }

    #[test]
    fn test_missing_node_resolution() {
        let mut graph = GridGraph::new();
        let mut n1 = GridNode::new("A".to_string(), "A".to_string(), "p".to_string(), "url".to_string());
        n1.dependencies.push("NON_EXISTENT".to_string());
        graph.add_node(n1);

        graph.compute_schedule();
        assert_eq!(graph.nodes.get("A").unwrap().start_time_offset, 0.0);
    }
}
