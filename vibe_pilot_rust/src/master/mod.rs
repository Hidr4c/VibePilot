pub mod dag;
pub mod ui;
pub mod prompt_compiler;
pub mod executor;

use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use dag::GridGraph;
use crate::config::ConfigurationRepository;

/// State structure for the VibePilot Master Orchestrator GUI.
pub struct MasterApp {
    pub graph: Arc<Mutex<GridGraph>>,
    pub selected_node_id: Option<String>,
    pub llm_prompt: String,
    pub is_compiling: bool,
    pub message: String,
    pub config_repo: Arc<dyn ConfigurationRepository>,
    pub rt: tokio::runtime::Runtime,
    pub compiler_tx: flume::Sender<Result<GridGraph, String>>,
    pub compiler_rx: flume::Receiver<Result<GridGraph, String>>,
    pub node_logs: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub worker_statuses: Arc<Mutex<HashMap<String, String>>>,
}

impl MasterApp {
    /// Constructs a new MasterApp instance.
    pub fn new(
        _cc: &egui::Context,
        config_repo: Arc<dyn ConfigurationRepository>,
        rt: tokio::runtime::Runtime,
    ) -> Self {
        let (compiler_tx, compiler_rx) = flume::unbounded();
        let mut app = Self {
            graph: Arc::new(Mutex::new(GridGraph::new())),
            selected_node_id: None,
            llm_prompt: String::new(),
            is_compiling: false,
            message: "Welcome to VibePilot Grid Master".to_string(),
            config_repo,
            rt,
            compiler_tx,
            compiler_rx,
            node_logs: Arc::new(Mutex::new(HashMap::new())),
            worker_statuses: Arc::new(Mutex::new(HashMap::new())),
        };

        // Initialize with a default demonstration DAG
        app.initialize_demo_graph();
        app
    }

    /// Spawns an async task to stream logs from a worker node in real time.
    pub fn start_log_stream(&self, node_id: String, worker_url: String) {
        let node_logs = self.node_logs.clone();
        let rt = &self.rt;
        
        rt.spawn(async move {
            let (host_port, token) = parse_worker_url(&worker_url);
            if host_port.is_empty() {
                return;
            }
            
            if let Ok(mut stream) = tokio::net::TcpStream::connect(&host_port).await {
                use tokio::io::AsyncWriteExt;
                let mut auth_header = String::new();
                if let Some(ref t) = token {
                    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let sig = generate_signature(t, timestamp, "/stream_logs");
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
                            if let Ok(mut logs_map) = node_logs.lock() {
                                logs_map.entry(node_id.clone()).or_insert_with(Vec::new).push(log_text);
                            }
                        }
                        line.clear();
                    }
                }
            }
        });
    }

    fn initialize_demo_graph(&mut self) {
        use dag::GridNode;
        if let Ok(mut g) = self.graph.lock() {
            g.add_node(GridNode::new(
                "node_1".to_string(),
                "1. Start Server".to_string(),
                "Start Local API".to_string(),
                "http://127.0.0.1:4040".to_string(),
            ));
            g.add_node(GridNode::new(
                "node_2".to_string(),
                "2. Run Web Agent".to_string(),
                "Chrome Automator".to_string(),
                "http://127.0.0.1:4041".to_string(),
            ));
            g.add_node(GridNode::new(
                "node_3".to_string(),
                "3. Notify Teams".to_string(),
                "Slack Alert".to_string(),
                "http://127.0.0.1:4040".to_string(),
            ));

            g.add_dependency("node_1", "node_2");
            g.add_dependency("node_2", "node_3");
            g.compute_schedule();
        }
    }
}

impl eframe::App for MasterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll compiler channel for background task updates
        if let Ok(result) = self.compiler_rx.try_recv() {
            self.is_compiling = false;
            match result {
                Ok(graph) => {
                    if let Ok(mut g) = self.graph.lock() {
                        *g = graph;
                    }
                    self.message = "DAG successfully compiled!".to_string();
                }
                Err(e) => {
                    self.message = format!("Compilation failed: {}", e);
                }
            }
        }

        ui::render_master_gui(ctx, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::factory::ConfigRepositoryFactory;

    #[test]
    fn test_master_app_lifecycle() {
        let egui_ctx = egui::Context::default();
        let temp_dir = std::env::temp_dir().join("vibepilot_master_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let config_repo = ConfigRepositoryFactory::create(temp_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut app = MasterApp::new(&egui_ctx, config_repo, rt);

        // Verify initial demo graph
        assert!(!app.graph.lock().unwrap().nodes.is_empty());
        assert_eq!(app.selected_node_id, None);
        assert!(!app.is_compiling);

        // Run UI rendering
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            ui::render_master_gui(ctx, &mut app);
        });

        // Test with a selected node
        let first_id = {
            let g = app.graph.lock().unwrap();
            g.nodes.keys().next().cloned()
        };
        if let Some(fid) = first_id {
            app.selected_node_id = Some(fid);
            let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
                ui::render_master_gui(ctx, &mut app);
            });
        }

        // Test compile trigger
        app.llm_prompt = "Start worker at http://127.0.0.1:4040".to_string();
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            ui::compile_dag_via_prompt(ctx, &mut app);
        });
        assert!(app.is_compiling);

        // Mock channel for testing polling inside update
        let (tx, rx) = flume::unbounded();
        app.compiler_rx = rx;

        // Test success compilation poll
        let current_graph = {
            let g = app.graph.lock().unwrap();
            g.clone()
        };
        tx.send(Ok(current_graph)).unwrap();
        let mut dummy_frame = std::mem::MaybeUninit::<eframe::Frame>::uninit();
        let frame: &mut eframe::Frame = unsafe { &mut *dummy_frame.as_mut_ptr() };
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            <MasterApp as eframe::App>::update(&mut app, ctx, frame);
        });
        assert!(!app.is_compiling);
        assert_eq!(app.message, "DAG successfully compiled!");

        // Test failed compilation poll
        tx.send(Err("LLM offline".to_string())).unwrap();
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            <MasterApp as eframe::App>::update(&mut app, ctx, frame);
        });
        assert!(!app.is_compiling);
        assert!(app.message.contains("Compilation failed: LLM offline"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

/// Helper to parse worker url and extract host/port and security token.
pub fn parse_worker_url(url: &str) -> (String, Option<String>) {
    let stripped = url.replace("http://", "").replace("https://", "");
    let mut token = None;
    let main_part = if let Some(pos) = stripped.find('?') {
        let query = &stripped[pos + 1..];
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let Some(key) = parts.next() {
                if key == "token" {
                    token = parts.next().map(|v| v.to_string());
                }
            }
        }
        &stripped[..pos]
    } else {
        &stripped
    };
    
    let (host_port, token_opt) = if let Some(pos) = main_part.find('@') {
        let tok = &main_part[..pos];
        let host = &main_part[pos + 1..];
        (host.to_string(), Some(tok.to_string()))
    } else {
        (main_part.to_string(), token)
    };
    
    let host_port_clean = host_port.split('/').next().unwrap_or("").to_string();
    (host_port_clean, token_opt)
}

/// Helper to generate a secure signature for a request path and timestamp.
pub fn generate_signature(secret: &str, timestamp: u64, path: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    let data = format!("{}:{}:{}", secret, timestamp, path);
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
