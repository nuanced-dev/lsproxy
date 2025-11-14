use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info, warn};
use lsproxy_common::api_types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};

/// Forward LSP JSON-RPC requests to language servers
#[utoipa::path(
    post,
    path = "/lsp",
    tag = "lsp",
    request_body = JsonRpcRequest,
    responses(
        (status = 200, description = "LSP request processed successfully", body = JsonRpcResponse),
        (status = 400, description = "Bad request - invalid JSON-RPC format"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn lsp(data: Data<AppState>, request: Json<JsonRpcRequest>) -> HttpResponse {
    let lsp_req = request.into_inner();

    // Convert LspRequest to Value for internal processing
    let req_value = serde_json::to_value(&lsp_req).unwrap_or_else(|_| json!({}));

    let method = &lsp_req.method;
    info!("Received LSP request: method={}", method);

    // Handle lifecycle requests locally
    if is_lifecycle_method(method) {
        return handle_lifecycle_request(&req_value, method);
    }

    // For language feature requests, extract document URI and route to appropriate backend
    let document_uri = match extract_document_uri(&req_value) {
        Some(uri) => uri,
        None => {
            warn!(
                "Could not extract document URI from request for method: {}",
                method
            );
            return HttpResponse::BadRequest().json(create_error_response(
                req_value.get("id").and_then(|id| id.as_u64()),
                -32602,
                "Invalid params: could not extract document URI",
            ));
        }
    };

    // Convert file:// URI to path
    let file_path = match uri_to_path(&document_uri) {
        Some(path) => path,
        None => {
            error!("Invalid document URI: {}", document_uri);
            return HttpResponse::BadRequest().json(create_error_response(
                req_value.get("id").and_then(|id| id.as_u64()),
                -32602,
                "Invalid params: invalid document URI",
            ));
        }
    };

    // Get container client for this file's language
    let client = match container_proxy::get_client_for_file(&data.orchestrator, &file_path).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to get container client: {}", e);
            return HttpResponse::InternalServerError().json(create_error_response(
                req_value.get("id").and_then(|id| id.as_u64()),
                -32603,
                &format!("Internal error: {}", e),
            ));
        }
    };

    // Forward request to container
    match client.forward_lsp_request(&req_value).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(create_error_response(
                req_value.get("id").and_then(|id| id.as_u64()),
                -32603,
                &format!("Internal error: {}", e),
            ))
        }
    }
}

/// Check if a method is a lifecycle method that should be handled by the orchestrator
fn is_lifecycle_method(method: &str) -> bool {
    matches!(
        method,
        "initialize" | "initialized" | "shutdown" | "exit" | "$/cancelRequest" | "$/setTrace"
    )
}

/// Handle lifecycle requests locally
fn handle_lifecycle_request(request: &Value, method: &str) -> HttpResponse {
    let id = request.get("id").and_then(|id| id.as_u64());

    match method {
        "initialize" => {
            // Return capabilities advertised by the orchestrator
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "hoverProvider": true,
                        "declarationProvider": true,
                        "definitionProvider": true,
                        "typeDefinitionProvider": true,
                        "referencesProvider": true,
                        "documentSymbolProvider": true,
                        "typeHierarchyProvider": true,
                    },
                    "serverInfo": {
                        "name": "nuanced-lsp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            });
            HttpResponse::Ok().json(response)
        }
        "initialized" => {
            // Notification - no response needed
            HttpResponse::Ok().finish()
        }
        "shutdown" => {
            // Acknowledge shutdown
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            });
            HttpResponse::Ok().json(response)
        }
        "exit" => {
            // Notification - no response needed
            HttpResponse::Ok().finish()
        }
        "$/cancelRequest" | "$/setTrace" => {
            // These are notifications or special requests - acknowledge
            HttpResponse::Ok().finish()
        }
        _ => {
            // Unknown lifecycle method
            HttpResponse::BadRequest().json(create_error_response(id, -32601, "Method not found"))
        }
    }
}

/// Extract document URI from JSON-RPC request parameters
fn extract_document_uri(request: &Value) -> Option<String> {
    // Try textDocument.uri
    if let Some(uri) = request
        .get("params")
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
    {
        return Some(uri.to_string());
    }

    // Try uri directly
    if let Some(uri) = request
        .get("params")
        .and_then(|p| p.get("uri"))
        .and_then(|u| u.as_str())
    {
        return Some(uri.to_string());
    }

    None
}

/// Convert file:// URI to file path
fn uri_to_path(uri: &str) -> Option<String> {
    if !uri.starts_with("file://") {
        return None;
    }

    // Remove file:// prefix
    let path = &uri[7..];

    // Handle Windows-style paths (file:///C:/...)
    if path.starts_with('/') && path.len() > 2 && path.chars().nth(2) == Some(':') {
        return Some(path[1..].to_string());
    }

    // Unix-style paths
    Some(path.to_string())
}

/// Create a JSON-RPC error response
fn create_error_response(id: Option<u64>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}
