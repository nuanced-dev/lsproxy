use crate::lsp::client::LspClient;
use actix_ws::{Message as WsMessage, MessageStream, Session};
use common::api_types::{JsonRpcErrorCode, JsonRpcMessage};
use futures::StreamExt;
use log::{debug, error, warn};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Manages WebSocket connections for bidirectional LSP communication
pub struct WsManager {
    client: Arc<Mutex<Box<dyn LspClient>>>,
}

impl WsManager {
    pub fn new(client: Arc<Mutex<Box<dyn LspClient>>>) -> Self {
        Self { client }
    }

    /// Handle a WebSocket connection, forwarding messages between WebSocket and LSP
    pub async fn handle_connection(
        &self,
        mut session: Session,
        mut msg_stream: MessageStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("New WebSocket connection established");

        spawn_notification_forwarder(self.client.clone(), session.clone());

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                WsMessage::Text(text) => {
                    if let Err(e) =
                        handle_ws_text_message(text.to_string(), &self.client, &session).await
                    {
                        error!("Error handling text message: {}", e);
                        break;
                    }
                }
                WsMessage::Ping(bytes) => {
                    debug!("Received ping");
                    if let Err(e) = session.pong(&bytes).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                WsMessage::Pong(_) => {
                    debug!("Received pong");
                }
                WsMessage::Close(reason) => {
                    debug!("WebSocket closed: {:?}", reason);
                    break;
                }
                _ => {
                    warn!("Received unsupported WebSocket message type");
                }
            }
        }

        debug!("WebSocket connection ended");
        Ok(())
    }
}

/// Spawn a task to forward LSP notifications to the WebSocket
fn spawn_notification_forwarder(client: Arc<Mutex<Box<dyn LspClient>>>, mut session: Session) {
    tokio::spawn(async move {
        let mut notification_rx = {
            let locked_client = client.lock().await;
            locked_client.subscribe_to_unexpected_notifications()
        };

        while let Ok(notification) = notification_rx.recv().await {
            debug!(
                "Forwarding notification to WebSocket: {}",
                notification
                    .method
                    .as_ref()
                    .unwrap_or(&"unknown".to_string())
            );

            match serde_json::to_string(&notification) {
                Ok(json) => {
                    if let Err(e) = session.text(json).await {
                        error!("Failed to send notification to WebSocket: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize notification: {}", e);
                }
            }
        }
        debug!("Notification forwarding task ended");
    });
}

/// Handle a text message from the WebSocket
async fn handle_ws_text_message(
    text: String,
    client: &Arc<Mutex<Box<dyn LspClient>>>,
    session: &Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Received WebSocket message: {}", text);

    let json_rpc_msg: JsonRpcMessage = serde_json::from_str(&text)?;

    let method = json_rpc_msg
        .method
        .clone()
        .ok_or("Received message without method field")?;

    if json_rpc_msg.id.is_some() {
        handle_lsp_request(json_rpc_msg, method, client.clone(), session.clone());
    } else {
        handle_lsp_notification(method, json_rpc_msg.params, client).await?;
    }

    Ok(())
}

/// Spawn a task to handle an LSP request and send the response
fn handle_lsp_request(
    json_rpc_msg: JsonRpcMessage,
    method: String,
    client: Arc<Mutex<Box<dyn LspClient>>>,
    mut session: Session,
) {
    tokio::spawn(async move {
        let request_id = json_rpc_msg.id.clone();

        // Send request to LSP server and wait for response
        let response = {
            let mut locked_client = client.lock().await;
            match locked_client
                .send_request(&method, json_rpc_msg.params)
                .await
            {
                Ok(result) => JsonRpcMessage::new_result_response(request_id, result),
                Err(e) => {
                    error!("LSP request failed: {}", e);
                    JsonRpcMessage::new_error_response(
                        request_id,
                        JsonRpcErrorCode::InternalError as i32,
                        e.to_string(),
                    )
                }
            }
        };

        // Send response back to WebSocket
        match serde_json::to_string(&response) {
            Ok(json) => {
                if let Err(e) = session.text(json).await {
                    error!("Failed to send response to WebSocket: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to serialize response: {}", e);
            }
        }
    });
}

/// Handle an LSP notification (no response expected)
async fn handle_lsp_notification(
    method: String,
    params: Option<serde_json::Value>,
    client: &Arc<Mutex<Box<dyn LspClient>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut locked_client = client.lock().await;
    locked_client.send_notification(&method, params).await?;
    Ok(())
}
