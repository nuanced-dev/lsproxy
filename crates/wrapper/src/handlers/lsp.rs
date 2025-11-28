use crate::AppState;
use actix_web::{web, HttpResponse};
use common::api_types::JsonRpcMessage;
use log::{debug, error, info};

/// Forward raw LSP JSON-RPC requests to the LSP server
///
/// This endpoint provides direct access to the LSP server by forwarding
/// JSON-RPC requests and returning responses with minimal processing.
#[utoipa::path(
    post,
    path = "/lsp",
    request_body = JsonRpcMessage,
    responses(
        (status = 200, description = "Identifier retrieved successfully", body = JsonRpcMessage),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn lsp(
    app_state: web::Data<AppState>,
    request: web::Json<JsonRpcMessage>,
) -> HttpResponse {
    let lsp_req = request.into_inner();
    let req_id = lsp_req.id.clone();

    info!(
        "Received LSP request: id={:?} method={:?}",
        &req_id, &lsp_req.method
    );
    debug!("LSP request: {:?}", &lsp_req);

    // Forward the request to the LSP server
    match app_state.manager.lsp(lsp_req).await {
        Ok(response) => {
            info!("Received process response: id={:?}", response.id);
            debug!("Process response: {:?}", response);
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("LSP request failed: {}", e);
            let error = JsonRpcMessage::new_error_response(
                req_id,
                -32603,
                format!("Internal error: {}", e),
            );
            HttpResponse::InternalServerError().json(error)
        }
    }
}
