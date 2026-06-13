use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::event_bus::{EventBus, CommandEvent, QueryEvent, EventType};

/// Lightweight, self-contained HTTP server to control and monitor the worker remotely.
pub struct WorkerApiServer {
    bus: EventBus,
    port: u16,
    token: Option<String>,
}

impl WorkerApiServer {
    /// Creates a new API server.
    pub fn new(bus: EventBus, port: u16, token: Option<String>) -> Self {
        Self { bus, port, token }
    }

    /// Spawns the server task on the async runtime.
    pub async fn start(&self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

        let bus = self.bus.clone();
        let token_clone = self.token.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut socket, _)) => {
                        let bus_clone = bus.clone();
                        let expected_token = token_clone.clone();
                        tokio::spawn(async move {
                            let mut buffer = [0; 1024];
                            if let Ok(size) = socket.read(&mut buffer).await {
                                let request = String::from_utf8_lossy(&buffer[..size]);
                                
                                if !verify_token(&request, &expected_token) {
                                    let response = "HTTP/1.1 401 UNAUTHORIZED\r\nContent-Type: application/json\r\nContent-Length: 26\r\nConnection: close\r\n\r\n{\"error\": \"Unauthorized\"}";
                                    let _ = socket.write_all(response.as_bytes()).await;
                                    return;
                                }
                                if request.starts_with("GET /stream_logs") {
                                    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
                                    if socket.write_all(headers.as_bytes()).await.is_ok() {
                                        let (tx, rx) = flume::unbounded::<String>();
                                        bus_clone.subscribe(Box::new(move |event| {
                                            if let EventType::Log(msg) = event {
                                                let _ = tx.send(msg.clone());
                                            }
                                        }));
                                        while let Ok(msg) = rx.recv_async().await {
                                            let sse_event = format!("data: {}\n\n", msg);
                                            if socket.write_all(sse_event.as_bytes()).await.is_err() {
                                                break;
                                            }
                                            let _ = socket.flush().await;
                                        }
                                    }
                                    return;
                                }
                                let (status_line, response_body) = if request.starts_with("GET /status") {
                                    let running = bus_clone.emit_query(QueryEvent::GetOrchestratorRunning) == "true";
                                    let paused = bus_clone.emit_query(QueryEvent::GetModePauseForcee) == "true";
                                    let state_str = if running {
                                        if paused { "Paused" } else { "Running" }
                                    } else {
                                        "Idle"
                                    };
                                    ("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n", format!("{{\"state\": \"{}\"}}", state_str))
                                } else if request.starts_with("POST /run") {
                                    let running = bus_clone.emit_query(QueryEvent::GetOrchestratorRunning) == "true";
                                    if running {
                                        ("HTTP/1.1 409 CONFLICT\r\nContent-Type: application/json\r\n", "{\"error\": \"Worker is busy running another session\"}".to_string())
                                    } else {
                                        let profile_name = if let Some(body_start) = request.find("\r\n\r\n") {
                                            let body = &request[body_start + 4..];
                                            if let Some(p_start) = body.find("\"profile\"") {
                                                let sub = &body[p_start..];
                                                if let Some(val_start) = sub.find(":") {
                                                    let val_sub = &sub[val_start + 1..];
                                                    let trimmed = val_sub.trim_matches(|c| c == ' ' || c == '"' || c == '}' || c == '\n' || c == '\r');
                                                    trimmed.to_string()
                                                } else {
                                                    String::new()
                                                }
                                            } else {
                                                String::new()
                                            }
                                        } else {
                                            String::new()
                                        };

                                        if !profile_name.is_empty() {
                                            bus_clone.emit_command(CommandEvent::LoadProfile(profile_name));
                                        }
                                        bus_clone.emit_command(CommandEvent::StartOrchestrator);
                                        ("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n", "{\"status\": \"started\"}".to_string())
                                    }
                                } else if request.starts_with("POST /stop") {
                                    bus_clone.emit_command(CommandEvent::StopOrchestrator);
                                    ("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n", "{\"status\": \"stopped\"}".to_string())
                                } else {
                                    ("HTTP/1.1 404 NOT FOUND\r\nContent-Type: application/json\r\n", "{\"error\": \"not found\"}".to_string())
                                };

                                let response = format!(
                                    "{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    status_line,
                                    response_body.len(),
                                    response_body
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        });
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use tokio::io::{AsyncWriteExt, AsyncReadExt};

    #[tokio::test]
    async fn test_real_worker_api_server() {
        let (bus, _rx) = EventBus::new();
        bus.register_query(
            QueryEvent::GetOrchestratorRunning,
            Box::new(|_| "false".to_string()),
        );
        bus.register_query(
            QueryEvent::GetModePauseForcee,
            Box::new(|_| "false".to_string()),
        );

        // Pick a port that is unlikely to be in use (e.g. 29845)
        let server = WorkerApiServer::new(bus.clone(), 29845, None);
        server.start().await.unwrap();

        // Give the listener a tiny fraction of time to bind
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 1. Test GET /status
        let mut stream = TcpStream::connect("127.0.0.1:29845").await.unwrap();
        stream.write_all(b"GET /status HTTP/1.1\r\n\r\n").await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 200 OK"));
        assert!(resp.contains("{\"state\": \"Idle\"}"));

        // 2. Test POST /run
        let mut stream = TcpStream::connect("127.0.0.1:29845").await.unwrap();
        stream.write_all(b"POST /run HTTP/1.1\r\n\r\n{\"profile\":\"test\"}").await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 200 OK"));
        assert!(resp.contains("started"));

        // 3. Test POST /stop
        let mut stream = TcpStream::connect("127.0.0.1:29845").await.unwrap();
        stream.write_all(b"POST /stop HTTP/1.1\r\n\r\n").await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 200 OK"));
        assert!(resp.contains("stopped"));

        // 4. Test GET /invalid
        let mut stream = TcpStream::connect("127.0.0.1:29845").await.unwrap();
        stream.write_all(b"GET /invalid HTTP/1.1\r\n\r\n").await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 404 NOT FOUND"));
    }

    #[tokio::test]
    async fn test_worker_api_server_token_auth() {
        let (bus, _rx) = EventBus::new();
        bus.register_query(
            QueryEvent::GetOrchestratorRunning,
            Box::new(|_| "false".to_string()),
        );
        bus.register_query(
            QueryEvent::GetModePauseForcee,
            Box::new(|_| "false".to_string()),
        );

        let server = WorkerApiServer::new(bus.clone(), 29849, Some("secret_key".to_string()));
        server.start().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let correct_sig = generate_signature_helper("secret_key", timestamp, "/status");

        // 1. Request with correct signature
        let mut stream = TcpStream::connect("127.0.0.1:29849").await.unwrap();
        let req1 = format!("GET /status HTTP/1.1\r\nX-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n\r\n", timestamp, correct_sig);
        stream.write_all(req1.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 200 OK"));
        assert!(resp.contains("{\"state\": \"Idle\"}"));

        // 2. Request with wrong signature
        let mut stream = TcpStream::connect("127.0.0.1:29849").await.unwrap();
        let req2 = format!("GET /status HTTP/1.1\r\nX-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n\r\n", timestamp, "wrongsignature");
        stream.write_all(req2.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 401 UNAUTHORIZED"));
        assert!(resp.contains("Unauthorized"));

        // 3. Request with expired timestamp
        let mut stream = TcpStream::connect("127.0.0.1:29849").await.unwrap();
        let expired_timestamp = timestamp - 120; // 2 minutes ago
        let expired_sig = generate_signature_helper("secret_key", expired_timestamp, "/status");
        let req3 = format!("GET /status HTTP/1.1\r\nX-VibePilot-Timestamp: {}\r\nX-VibePilot-Signature: {}\r\n\r\n", expired_timestamp, expired_sig);
        stream.write_all(req3.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(resp.contains("HTTP/1.1 401 UNAUTHORIZED"));
    }

    #[tokio::test]
    async fn test_stream_logs() {
        let (bus, _rx) = EventBus::new();
        let server = WorkerApiServer::new(bus.clone(), 29846, None);
        server.start().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect("127.0.0.1:29846").await.unwrap();
        stream.write_all(b"GET /stream_logs HTTP/1.1\r\n\r\n").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        bus.emit_notification(crate::event_bus::NotificationEvent::Log("Hello from test!".to_string()));

        let mut resp = String::new();
        let mut buf = [0; 1024];
        for _ in 0..20 {
            if let Ok(size) = stream.read(&mut buf).await {
                if size == 0 {
                    break;
                }
                resp.push_str(&String::from_utf8_lossy(&buf[..size]));
                if resp.contains("data: Hello from test!") {
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }

        assert!(resp.contains("HTTP/1.1 200 OK"));
        assert!(resp.contains("Content-Type: text/event-stream"));
        assert!(resp.contains("data: Hello from test!"));
    }
}

fn verify_token(request: &str, expected_token: &Option<String>) -> bool {
    if let Some(ref secret) = expected_token {
        let first_line = request.lines().next().unwrap_or("");
        let path = if first_line.starts_with("GET ") {
            first_line.trim_start_matches("GET ").split(' ').next().unwrap_or("")
        } else if first_line.starts_with("POST ") {
            first_line.trim_start_matches("POST ").split(' ').next().unwrap_or("")
        } else {
            ""
        };
        
        let path_clean = path.split('?').next().unwrap_or("");

        let mut timestamp_opt: Option<u64> = None;
        let mut signature_opt: Option<String> = None;

        for line in request.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("x-vibepilot-timestamp:") {
                let val = line.splitn(2, ':').nth(1).unwrap_or("").trim();
                timestamp_opt = val.parse::<u64>().ok();
            } else if lower.starts_with("x-vibepilot-signature:") {
                let val = line.splitn(2, ':').nth(1).unwrap_or("").trim();
                signature_opt = Some(val.to_string());
            }
        }

        if timestamp_opt.is_none() || signature_opt.is_none() {
            if let Some(pos) = path.find('?') {
                let query = &path[pos + 1..];
                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    if let Some(key) = parts.next() {
                        if key == "timestamp" {
                            timestamp_opt = parts.next().and_then(|v| v.parse::<u64>().ok());
                        } else if key == "signature" {
                            signature_opt = parts.next().map(|v| v.to_string());
                        }
                    }
                }
            }
        }

        if let (Some(timestamp), Some(signature)) = (timestamp_opt, signature_opt) {
            let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let diff = if current_time > timestamp { current_time - timestamp } else { timestamp - current_time };
            if diff > 60 {
                return false;
            }

            let expected_sig = generate_signature_helper(secret, timestamp, path_clean);
            if signature == expected_sig {
                return true;
            }
        }
        
        false
    } else {
        true
    }
}

fn generate_signature_helper(secret: &str, timestamp: u64, path: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    let data = format!("{}:{}:{}", secret, timestamp, path);
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

