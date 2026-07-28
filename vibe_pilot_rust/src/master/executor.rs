//! VibePilot parallel DAG executor.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::master::dag::{GridGraph, GridNode, NodeState};

/// Asynchronous DAG scheduler and execution coordinator.
pub struct GridGraphExecutor {
    graph: Arc<Mutex<GridGraph>>,
    node_logs: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl GridGraphExecutor {
    /// Constructs a new executor referencing shared state.
    pub fn new(graph: Arc<Mutex<GridGraph>>, node_logs: Arc<Mutex<HashMap<String, Vec<String>>>>) -> Self {
        Self { graph, node_logs }
    }

    /// Spawns the execution loop on the tokio runtime handle.
    pub fn start_execution(&self, rt: &tokio::runtime::Handle) {
        let graph = self.graph.clone();
        let node_logs = self.node_logs.clone();
        
        rt.spawn(async move {
            // Reset all nodes to Pending
            {
                if let Ok(mut g) = graph.lock() {
                    for node in g.nodes.values_mut() {
                        node.state = NodeState::Pending;
                    }
                }
            }

            loop {
                let mut all_done = true;
                let mut has_running = false;
                let mut to_start = Vec::new();

                // Check node states and dependencies
                {
                    if let Ok(g) = graph.lock() {
                        for (id, node) in &g.nodes {
                            match node.state {
                                NodeState::Pending => {
                                    all_done = false;
                                    let mut deps_satisfied = true;
                                    for dep in &node.dependencies {
                                        if let Some(dep_node) = g.nodes.get(dep) {
                                            if dep_node.state != NodeState::Success {
                                                deps_satisfied = false;
                                                break;
                                            }
                                        }
                                    }
                                    if deps_satisfied {
                                        to_start.push((id.clone(), node.worker_url.clone(), node.profile_name.clone()));
                                    }
                                }
                                NodeState::Running => {
                                    all_done = false;
                                    has_running = true;
                                }
                                NodeState::Failed => {
                                    // Cascade block to dependents
                                }
                                NodeState::Success => {}
                            }
                        }
                    }
                }

                // Trigger eligible nodes
                for (id, url, profile) in to_start {
                    {
                        if let Ok(mut g) = graph.lock() {
                            if let Some(node) = g.nodes.get_mut(&id) {
                                node.state = NodeState::Running;
                            }
                        }
                    }

                    // Start streaming logs in background
                    let node_logs_clone = node_logs.clone();
                    let id_clone = id.clone();
                    let url_clone = url.clone();
                    tokio::spawn(async move {
                        let (host_port, token) = crate::master::parse_worker_url(&url_clone);
                        if let Ok(mut stream) = tokio::net::TcpStream::connect(&host_port).await {
                            use tokio::io::AsyncWriteExt;
                            let mut auth_header = String::new();
                            if let Some(ref t) = token {
                                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                let sig = crate::master::generate_signature(t, timestamp, "/stream_logs");
                                auth_header = format!("X-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n", timestamp, sig);
                            }
                            let request = format!("GET /stream_logs HTTP/1.1\r\n{}Connection: keep-alive\r\n\r\n", auth_header);
                            if stream.write_all(request.as_bytes()).await.is_ok() {
                                let mut reader = tokio::io::BufReader::new(stream);
                                use tokio::io::AsyncBufReadExt;
                                let mut line = String::new();
                                while let Ok(bytes) = reader.read_line(&mut line).await {
                                    if bytes == 0 {
                                        break;
                                    }
                                    if line.starts_with("data: ") {
                                        let log_text = line.trim_start_matches("data: ").trim().to_string();
                                        if let Ok(mut logs_map) = node_logs_clone.lock() {
                                            logs_map.entry(id_clone.clone()).or_insert_with(Vec::new).push(log_text);
                                        }
                                    }
                                    line.clear();
                                }
                            }
                        }
                    });

                    // Trigger run API request
                    let id_clone = id.clone();
                    let url_clone = url.clone();
                    tokio::spawn(async move {
                        let _ = trigger_node(id_clone, url_clone, profile).await;
                    });
                }

                // Poll running nodes status
                if has_running || !all_done {
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

                    let mut running_nodes = Vec::new();
                    {
                        if let Ok(g) = graph.lock() {
                            for (id, node) in &g.nodes {
                                if node.state == NodeState::Running {
                                    running_nodes.push((id.clone(), node.worker_url.clone()));
                                }
                            }
                        }
                    }

                    for (id, url) in running_nodes {
                        if let Ok(status) = query_status(url).await {
                            if status == "Idle" {
                                if let Ok(mut g) = graph.lock() {
                                    if let Some(node) = g.nodes.get_mut(&id) {
                                        node.state = NodeState::Success;
                                    }
                                }
                            } else if status == "Error" {
                                if let Ok(mut g) = graph.lock() {
                                    if let Some(node) = g.nodes.get_mut(&id) {
                                        node.state = NodeState::Failed;
                                    }
                                }
                            }
                        }
                    }
                }

                if all_done && !has_running {
                    break;
                }
            }
        });
    }
}

async fn trigger_node(_node_id: String, worker_url: String, profile_name: String) -> Result<(), String> {
    let (host_port, token) = crate::master::parse_worker_url(&worker_url);
    if host_port.is_empty() {
        return Err("Empty worker host/port".to_string());
    }
    
    if let Ok(mut stream) = tokio::net::TcpStream::connect(&host_port).await {
        use tokio::io::AsyncWriteExt;
        let body = format!("{{\"profile\":\"{}\"}}", profile_name);
        let mut auth_header = String::new();
        if let Some(ref t) = token {
            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let sig = crate::master::generate_signature(t, timestamp, "/run");
            auth_header = format!("X-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n", timestamp, sig);
        }
        let request = format!(
            "POST /run HTTP/1.1\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            auth_header,
            body.len(),
            body
        );
        let _ = stream.write_all(request.as_bytes()).await;
    }
    Ok(())
}

async fn query_status(worker_url: String) -> Result<String, String> {
    let (host_port, token) = crate::master::parse_worker_url(&worker_url);
    if host_port.is_empty() {
        return Err("Offline".to_string());
    }
    
    if let Ok(mut stream) = tokio::net::TcpStream::connect(&host_port).await {
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
            let _ = stream.read_to_string(&mut resp).await;
            if resp.contains("Running") {
                return Ok("Running".to_string());
            } else if resp.contains("Paused") {
                return Ok("Paused".to_string());
            } else if resp.contains("Idle") {
                return Ok("Idle".to_string());
            }
        }
    }
    Err("Offline".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::api::WorkerApiServer;
    use crate::event_bus::EventBus;

    #[tokio::test]
    async fn test_graph_executor_workflow() {
        let (bus, _rx) = EventBus::new();
        bus.register_query(
            crate::event_bus::QueryEvent::GetOrchestratorRunning,
            Box::new(|_| "false".to_string()),
        );
        bus.register_query(
            crate::event_bus::QueryEvent::GetModePauseForcee,
            Box::new(|_| "false".to_string()),
        );

        let server = WorkerApiServer::new(bus, 29847, None);
        server.start().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut graph = GridGraph::new();
        let n1 = GridNode::new("node1".to_string(), "Node 1".to_string(), "profile1".to_string(), "http://127.0.0.1:29847".to_string());
        graph.add_node(n1);

        let shared_graph = Arc::new(Mutex::new(graph));
        let shared_logs = Arc::new(Mutex::new(HashMap::new()));

        let executor = GridGraphExecutor::new(shared_graph.clone(), shared_logs);
        let rt = tokio::runtime::Handle::current();
        executor.start_execution(&rt);

        // Wait for executor execution to complete (with polling and timeout)
        let start = std::time::Instant::now();
        let mut final_state = NodeState::Pending;
        while start.elapsed() < std::time::Duration::from_secs(3) {
            final_state = {
                let g = shared_graph.lock().unwrap();
                g.nodes.get("node1").unwrap().state
            };
            if final_state == NodeState::Success || final_state == NodeState::Failed {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert_eq!(final_state, NodeState::Success);
    }
}
