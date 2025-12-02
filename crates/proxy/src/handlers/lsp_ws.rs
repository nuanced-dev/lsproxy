use crate::handlers::container_proxy;
use crate::handlers::lsp::{
    convert_json_paths_container_to_host, convert_json_paths_host_to_container,
    extract_document_uri, handle_lifecycle_request,
};
use crate::AppState;
use actix_web::web::{Data, Payload};
use actix_web::{HttpRequest, HttpResponse};
use actix_ws::{Message as WsMessage, MessageStream, Session};
use common::api_types::{JsonRpcMessage, SupportedLanguages};
use common::utils::language_utils::detect_language;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use url::Url;

type ContainerWsConnection = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type ContainerWsSink = SplitSink<ContainerWsConnection, TungsteniteMessage>;

/// WebSocket endpoint for bidirectional LSP communication
pub async fn lsp_ws(
    req: HttpRequest,
    stream: Payload,
    data: Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    debug!("WebSocket connection request received at proxy");

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Spawn task to handle the connection using actix_web::rt (supports !Send futures)
    actix_web::rt::spawn(async move {
        if let Err(e) = handle_ws_connection(session, msg_stream, data).await {
            error!("WebSocket connection error: {}", e);
        }
    });

    Ok(response)
}

async fn handle_ws_connection(
    mut client_session: Session,
    mut client_stream: MessageStream,
    data: Data<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Handling WebSocket connection");

    // Cache of container WebSocket sinks by language (for sending messages to containers)
    let container_sinks: Arc<Mutex<HashMap<SupportedLanguages, ContainerWsSink>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Process incoming messages from client
    while let Some(result) = client_stream.next().await {
        match result {
            Ok(WsMessage::Text(text)) => {
                if let Err(e) = handle_client_text_message(
                    text.to_string(),
                    &mut client_session,
                    &data,
                    &container_sinks,
                )
                .await
                {
                    error!("Error handling client text message: {}", e);
                    break;
                }
            }
            Ok(WsMessage::Ping(bytes)) => {
                if let Err(e) = client_session.pong(&bytes).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            Ok(WsMessage::Pong(_)) => {
                debug!("Received pong from client");
            }
            Ok(WsMessage::Close(reason)) => {
                debug!("Client closed connection: {:?}", reason);
                break;
            }
            Err(e) => {
                error!("Error receiving from client: {}", e);
                break;
            }
            _ => {}
        }
    }

    debug!("WebSocket connection ended");
    Ok(())
}

/// Handle a text message from the client
async fn handle_client_text_message(
    text: String,
    client_session: &mut Session,
    data: &Data<AppState>,
    container_sinks: &Arc<Mutex<HashMap<SupportedLanguages, ContainerWsSink>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Client -> Proxy: {}", text);

    // Parse JSON-RPC message
    let json_rpc_msg: JsonRpcMessage = serde_json::from_str(&text)?;

    // Handle lifecycle requests locally
    if let Ok(result) = handle_lifecycle_request(&json_rpc_msg) {
        if let Some(response) = result {
            let response_text = serde_json::to_string(&response)?;
            client_session.text(response_text).await?;
        }
        return Ok(());
    }

    // Extract document URI to determine routing
    let document_uri =
        extract_document_uri(&json_rpc_msg).ok_or("Could not extract document URI from message")?;

    // Convert file:// URI to path and detect language
    let file_path = uri_to_file_path(&document_uri)?;
    let language = detect_language(&file_path)
        .map_err(|e| format!("Failed to detect language for {}: {}", file_path, e))?;

    debug!(
        "Routing message for file {} (language: {:?})",
        file_path, language
    );

    // Convert paths and send message to container
    send_to_container(
        json_rpc_msg,
        language,
        data,
        client_session,
        container_sinks,
    )
    .await
}

/// Convert a file:// URI to a file path
fn uri_to_file_path(uri: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = Url::parse(uri)?;
    if url.scheme() != "file" {
        return Err(format!("Expected file URI, got: {}", uri).into());
    }
    let path = url
        .to_file_path()
        .map_err(|_| format!("Invalid file URI path: {}", uri))?;
    Ok(path.to_string_lossy().to_string())
}

/// Ensure a container connection exists for the given language and return the sink
async fn ensure_container_connection(
    language: SupportedLanguages,
    data: &Data<AppState>,
    client_session: &Session,
    container_sinks: &Arc<Mutex<HashMap<SupportedLanguages, ContainerWsSink>>>,
) -> Result<ContainerWsSink, Box<dyn std::error::Error + Send + Sync>> {
    let mut sinks = container_sinks.lock().await;

    // Return existing sink if available
    if sinks.contains_key(&language) {
        // We need to remove and return the sink to satisfy borrow checker
        // It will be re-inserted by the caller
        return Ok(sinks.remove(&language).unwrap());
    }

    // Get container client for this language
    let container_client =
        container_proxy::get_container_api_client(&data.orchestrator, language.clone()).await?;

    // Connect to container's WebSocket endpoint
    let container_ws_url = format!("{}/lsp/ws", container_client.get_base_url())
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    debug!("Connecting to container WebSocket: {}", container_ws_url);

    let (container_ws, _) = tokio_tungstenite::connect_async(&container_ws_url).await?;

    info!("Connected to container WebSocket for {:?}", language);

    // Split WebSocket into read and write halves
    let (sink, stream) = container_ws.split();

    // Spawn a task to read from this connection and forward to client
    spawn_container_reader(
        language.clone(),
        stream,
        client_session.clone(),
        data.orchestrator.clone(),
    );

    Ok(sink)
}

/// Spawn a task to read from a container WebSocket and forward messages to the client
fn spawn_container_reader(
    language: SupportedLanguages,
    stream: futures_util::stream::SplitStream<ContainerWsConnection>,
    mut client_session: Session,
    orchestrator: Arc<crate::container::ContainerOrchestrator>,
) {
    actix_web::rt::spawn(async move {
        let mut stream = stream;
        while let Some(msg_result) = stream.next().await {
            match msg_result {
                Ok(TungsteniteMessage::Text(text)) => {
                    debug!("Container -> Client from {:?}: {}", language, text);

                    if let Err(e) = forward_container_message_to_client(
                        text,
                        &mut client_session,
                        &orchestrator,
                    )
                    .await
                    {
                        error!("Failed to forward container message: {}", e);
                        break;
                    }
                }
                Ok(TungsteniteMessage::Close(_)) => {
                    debug!("Container WebSocket closed for {:?}", language);
                    break;
                }
                Err(e) => {
                    error!(
                        "Error reading from container WebSocket for {:?}: {}",
                        language, e
                    );
                    break;
                }
                _ => {}
            }
        }
    });
}

/// Convert paths in a container message and forward to client
async fn forward_container_message_to_client(
    text: String,
    client_session: &mut Session,
    orchestrator: &Arc<crate::container::ContainerOrchestrator>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse and convert paths
    match serde_json::from_str::<JsonRpcMessage>(&text) {
        Ok(mut msg) => {
            // Convert result paths
            if let Some(ref mut result) = msg.result {
                convert_json_paths_container_to_host(orchestrator, result).await?;
            }

            // Convert params paths (for notifications)
            if let Some(ref mut params) = msg.params {
                convert_json_paths_container_to_host(orchestrator, params).await?;
            }

            let converted_text = serde_json::to_string(&msg)?;
            client_session.text(converted_text).await?;
        }
        Err(e) => {
            warn!("Failed to parse message from container: {}", e);
            // Forward as-is if not valid JSON-RPC
            client_session.text(text).await?;
        }
    }
    Ok(())
}

/// Convert paths in a message and send to container
async fn send_to_container(
    json_rpc_msg: JsonRpcMessage,
    language: SupportedLanguages,
    data: &Data<AppState>,
    client_session: &Session,
    container_sinks: &Arc<Mutex<HashMap<SupportedLanguages, ContainerWsSink>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Convert paths in message
    let mut converted_msg = json_rpc_msg;
    if let Some(ref mut params) = converted_msg.params {
        convert_json_paths_host_to_container(&data.orchestrator, params).await?;
    }

    // Send to container
    let converted_text = serde_json::to_string(&converted_msg)?;

    // Get or create the sink for this language
    let mut sink =
        ensure_container_connection(language.clone(), data, client_session, container_sinks)
            .await?;

    // Send message to container
    let send_result = sink.send(TungsteniteMessage::Text(converted_text)).await;

    // Re-insert sink back into the map if successful, otherwise remove it
    match send_result {
        Ok(_) => {
            let mut sinks = container_sinks.lock().await;
            sinks.insert(language, sink);
            Ok(())
        }
        Err(e) => {
            // Don't re-insert the sink on error - it's likely broken
            Err(e.into())
        }
    }
}
