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
    stdin: Arc<Mutex<ChildStdin>>,
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

        let process = Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            request_id: Arc::new(Mutex::new(0)),
            pending_requests,
        };

        // Initialize the LSP server (matching main lsproxy's LspClient::initialize pattern)
        process.initialize(workspace_path).await?;

        Ok(process)
    }

    /// Initialize the LSP server - mirrors main lsproxy's LspClient::initialize
    async fn initialize(&self, workspace_path: &str) -> Result<(), std::io::Error> {
        info!("Initializing LSP server with root path: {:?}", workspace_path);

        // Build initialize params (matching get_initialize_params + get_capabilities)
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": format!("file://{}", workspace_path),
            "capabilities": {
                "textDocument": {
                    "documentSymbol": {
                        "dynamicRegistration": false,
                        "hierarchicalDocumentSymbolSupport": true
                    },
                    "publishDiagnostics": {
                        "relatedInformation": false,
                        "tagSupport": { "valueSet": [] },
                        "codeDescriptionSupport": false,
                        "dataSupport": false,
                        "versionSupport": false
                    }
                },
                "experimental": {
                    "serverStatusNotification": true
                }
            },
            "workspaceFolders": [{
                "uri": format!("file://{}", workspace_path),
                "name": "workspace"
            }]
        });

        // Send initialize request and wait for response
        let initialize_request = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None, // send_request will assign ID
            method: Some("initialize".to_string()),
            params: Some(params),
            result: None,
            error: None,
        };

        match self.send_request(&initialize_request).await {
            Ok(result) => {
                debug!("Initialization successful: {:?}", result);
                // Send initialized notification (matching send_initialized)
                self.send_initialized().await?;
                Ok(())
            }
            Err(e) => {
                error!("Failed to initialize LSP server: {}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    /// Send initialized notification - mirrors main lsproxy's send_initialized
    async fn send_initialized(&self) -> Result<(), std::io::Error> {
        debug!("Sending 'initialized' notification");

        let notification = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None, // Notifications have no ID
            method: Some("initialized".to_string()),
            params: Some(serde_json::json!({})),
            result: None,
            error: None,
        };

        let notification_json = serde_json::to_string(&notification)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", notification_json.len(), notification_json);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Send a JSON-RPC request to the LSP server and wait for response
    /// This method can be called concurrently - the lock is only held briefly during the write
    pub async fn send_request(
        &self,
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

        // Lock stdin only for the write operation (brief)
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(message.as_bytes()).await?;
            stdin.flush().await?;
        } // Lock released here

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
    /// Uses the same reading pattern as lsproxy/src/lsp/process.rs
    fn start_response_listener(stdout: ChildStdout, pending_requests: PendingRequests) {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();

            loop {
                let mut content_length: Option<usize> = None;

                // Read headers until we find Content-Length and empty line
                loop {
                    let n = match reader.read_until(b'\n', &mut buffer).await {
                        Ok(n) => n,
                        Err(e) => {
                            error!("Failed to read from LSP stdout: {}", e);
                            return;
                        }
                    };

                    if n == 0 {
                        buffer.clear();
                        continue;
                    }

                    let line = String::from_utf8_lossy(&buffer[buffer.len() - n..]);

                    // Check if this is the empty line separator
                    if line.trim().is_empty() && content_length.is_some() {
                        break; // Ready to read JSON body
                    } else if line.starts_with("Content-Length: ") {
                        match line.trim_start_matches("Content-Length: ").trim().parse::<usize>() {
                            Ok(len) => content_length = Some(len),
                            Err(_) => {
                                error!("Invalid Content-Length: {}", line);
                                buffer.clear();
                                continue;
                            }
                        }
                    }
                    buffer.clear();
                }

                // Read JSON body
                let length = match content_length {
                    Some(len) => len,
                    None => {
                        error!("Missing Content-Length header");
                        continue;
                    }
                };

                debug!("Reading JSON body of length: {}", length);
                let mut json_buffer = vec![0u8; length];
                if let Err(e) = reader.read_exact(&mut json_buffer).await {
                    error!("Failed to read JSON body: {}", e);
                    break;
                }

                let json_str = match String::from_utf8(json_buffer) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Invalid UTF-8 in JSON body: {}", e);
                        continue;
                    }
                };

                debug!("Read JSON string of length: {}", json_str.len());

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