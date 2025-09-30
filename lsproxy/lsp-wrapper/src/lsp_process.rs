use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

/// Represents a JSON-RPC message (request or response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Tracks pending requests waiting for responses
struct PendingRequests {
    channels: Arc<Mutex<HashMap<u64, Sender<JsonRpcMessage>>>>,
}

impl PendingRequests {
    fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn add_request(&self, id: u64) -> Result<Receiver<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        let (tx, rx) = channel::<JsonRpcMessage>(16);
        self.channels.lock().await.insert(id, tx);
        Ok(rx)
    }

    async fn remove_request(&self, id: u64) -> Option<Sender<JsonRpcMessage>> {
        self.channels.lock().await.remove(&id)
    }
}

impl Clone for PendingRequests {
    fn clone(&self) -> Self {
        Self {
            channels: Arc::clone(&self.channels),
        }
    }
}

/// Manages the LSP server process and handles JSON-RPC communication
pub struct LspProcess {
    child: Child,
    stdin: ChildStdin,
    request_id: Arc<Mutex<u64>>,
    pending_requests: PendingRequests,
}

impl LspProcess {
    /// Start a new LSP server process
    pub async fn new(
        command: &str,
        args: &[&str],
        workspace_path: &str,
    ) -> Result<Self, std::io::Error> {
        info!("Starting LSP process: {} {:?}", command, args);
        info!("Workspace: {}", workspace_path);

        let mut child = Command::new(command)
            .args(args)
            .current_dir(workspace_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit()) // Inherit stderr for logging
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to capture stdin")
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to capture stdout")
        })?;

        let pending_requests = PendingRequests::new();

        // Start the response listener task
        Self::start_response_listener(stdout, pending_requests.clone());

        Ok(Self {
            child,
            stdin,
            request_id: Arc::new(Mutex::new(0)),
            pending_requests,
        })
    }

    /// Send a JSON-RPC request to the LSP server and wait for response
    pub async fn send_request(
        &mut self,
        request: &JsonRpcMessage,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        // Assign an ID if not present
        let id = if let Some(id) = request.id {
            id
        } else {
            let mut current_id = self.request_id.lock().await;
            *current_id += 1;
            *current_id
        };

        let mut req = request.clone();
        req.id = Some(id);

        // Register channel to receive response
        let mut response_receiver = self.pending_requests.add_request(id).await?;

        // Serialize and send request
        let request_json = serde_json::to_string(&req)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", request_json.len(), request_json);

        debug!("Sending request {}: {}", id, request_json);
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;

        // Wait for response
        let response = response_receiver
            .recv()
            .await
            .ok_or("Failed to receive response")?;

        debug!("Received response for request {}", id);

        // Return result or error
        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(format!("LSP error: {:?}", error).into())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    /// Background task that reads from LSP stdout and routes responses
    fn start_response_listener(stdout: ChildStdout, pending_requests: PendingRequests) {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();

            loop {
                buffer.clear();

                // Read Content-Length header
                if reader.read_line(&mut buffer).await.is_err() {
                    error!("Failed to read from LSP stdout");
                    break;
                }

                if buffer.trim().is_empty() {
                    continue;
                }

                // Parse Content-Length
                let content_length = if let Some(len_str) = buffer.strip_prefix("Content-Length: ") {
                    match len_str.trim().parse::<usize>() {
                        Ok(len) => len,
                        Err(_) => {
                            error!("Invalid Content-Length: {}", buffer);
                            continue;
                        }
                    }
                } else {
                    continue;
                };

                // Read empty line
                buffer.clear();
                if reader.read_line(&mut buffer).await.is_err() {
                    break;
                }

                // Read JSON body
                let mut json_buffer = vec![0u8; content_length];
                if reader.read_exact(&mut json_buffer).await.is_err() {
                    error!("Failed to read JSON body");
                    break;
                }

                let json_str = match String::from_utf8(json_buffer) {
                    Ok(s) => s,
                    Err(_) => {
                        error!("Invalid UTF-8 in JSON body");
                        continue;
                    }
                };

                // Parse JSON-RPC message
                let message: JsonRpcMessage = match serde_json::from_str(&json_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Failed to parse JSON-RPC message: {} - {}", e, json_str);
                        continue;
                    }
                };

                // Route response to waiting channel
                if let Some(id) = message.id {
                    debug!("Routing response for request {}", id);
                    if let Some(sender) = pending_requests.remove_request(id).await {
                        if sender.send(message).await.is_err() {
                            error!("Failed to send response for request {}", id);
                        }
                    } else {
                        debug!("No pending request for id {}", id);
                    }
                } else {
                    // Notification from server (no response needed)
                    debug!("Received notification: {:?}", message.method);
                }
            }

            info!("Response listener stopped");
        });
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        info!("Stopping LSP process");
        let _ = self.child.start_kill();
    }
}