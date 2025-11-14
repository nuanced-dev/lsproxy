use crate::AppState;
use actix_web::{web, HttpResponse};
use log::{debug, error};
use lsproxy_common::api_types::JsonRpcRequest;

/// Forward raw LSP JSON-RPC requests to the LSP server
///
/// This endpoint provides direct access to the LSP server by forwarding
/// JSON-RPC requests and returning responses with minimal processing.
pub async fn lsp(
    app_state: web::Data<AppState>,
    request: web::Json<JsonRpcRequest>,
) -> HttpResponse {
    let lsp_req = request.into_inner();
    let req_value = serde_json::to_value(&lsp_req).unwrap_or_else(|_| serde_json::json!({}));

    debug!("Forwarding LSP request: {:?}", req_value);

    // Forward the request to the LSP server
    match app_state.manager.forward_lsp_request(req_value).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("LSP request failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32603,
                    "message": format!("Internal error: {}", e)
                }
            }))
        }
    }
}
