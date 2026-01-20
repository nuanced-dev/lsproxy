use crate::managers::ws::WsManager;
use actix_web::{web, HttpRequest, HttpResponse};
use log::{debug, error};

/// WebSocket endpoint for bidirectional LSP communication
pub async fn lsp_ws(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<crate::AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    debug!("WebSocket connection request received");

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Get the LSP client from the ApiManager
    let client = data.api_manager.get_lsp_client();

    // Create WsManager and handle the connection
    let ws_manager = WsManager::new(client);

    // Spawn task to handle the connection using actix_web::rt (supports !Send futures)
    actix_web::rt::spawn(async move {
        if let Err(e) = ws_manager.handle_connection(session, msg_stream).await {
            error!("WebSocket connection error: {}", e);
        }
    });

    Ok(response)
}
