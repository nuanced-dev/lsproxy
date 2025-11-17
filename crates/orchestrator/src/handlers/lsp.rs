use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info, warn};
use lsp_types::{
    DeclarationCapability, HoverProviderCapability, InitializeResult, OneOf, PositionEncodingKind,
    ServerCapabilities, ServerInfo,
};
use lsproxy_common::api_types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;

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

    let method = &lsp_req.method;
    let req_id = lsp_req.id.clone();
    info!("Received LSP request: id={:?} method={}", &req_id, method);

    // Handle lifecycle requests locally
    if is_lifecycle_method(method) {
        return handle_lifecycle_request(&lsp_req, method);
    }

    // For language feature requests, extract document URI and route to appropriate backend
    let document_uri = match extract_document_uri(lsp_req.params.as_ref()) {
        Some(uri) => uri,
        None => {
            warn!(
                "Could not extract document URI from request for method: {}",
                method
            );
            return HttpResponse::BadRequest().json(JsonRpcResponse::new_error(
                req_id,
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
            return HttpResponse::BadRequest().json(JsonRpcResponse::new_error(
                req_id,
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
            return HttpResponse::InternalServerError().json(JsonRpcResponse::new_error(
                req_id,
                -32603,
                &format!("Internal error: {}", e),
            ));
        }
    };

    // Forward request to container
    match client.forward_lsp_request(&lsp_req).await {
        Ok(response) => {
            info!(
                "Received container response: id={} method={}",
                response.id, method
            );
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(JsonRpcResponse::new_error(
                req_id,
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
fn handle_lifecycle_request(request: &JsonRpcRequest, method: &str) -> HttpResponse {
    let req_id = request.id.clone();
    match method {
        "initialize" => {
            // Return capabilities advertised by the orchestrator
            let mut capabilities = ServerCapabilities::default();
            capabilities.call_hierarchy_provider = Some(true.into());
            capabilities.declaration_provider = Some(DeclarationCapability::Simple(true));
            capabilities.definition_provider = Some(OneOf::Left(true));
            capabilities.document_symbol_provider = Some(OneOf::Left(true));
            capabilities.hover_provider = Some(HoverProviderCapability::Simple(false));
            capabilities.position_encoding = Some(PositionEncodingKind::UTF16);
            capabilities.references_provider = Some(OneOf::Left(true));
            capabilities.type_definition_provider = Some(true.into());
            let response = JsonRpcResponse::new_result(
                req_id,
                InitializeResult {
                    capabilities,
                    server_info: Some(ServerInfo {
                        name: "nuanced-lsp".to_string(),
                        version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    }),
                },
            );
            HttpResponse::Ok().json(response)
        }
        "initialized" => {
            // Notification - no response needed
            HttpResponse::Ok().finish()
        }
        "shutdown" => {
            // Acknowledge shutdown
            let response = JsonRpcResponse::new_result(req_id, Value::Null);
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
            HttpResponse::BadRequest().json(JsonRpcResponse::new_error(
                req_id,
                -32601,
                "Method not found",
            ))
        }
    }
}

/// Extract document URI from JSON-RPC request parameters
fn extract_document_uri(params: Option<&Value>) -> Option<String> {
    // Try textDocument.uri
    if let Some(uri) = params
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
    {
        return Some(uri.to_string());
    }

    // Try uri directly
    if let Some(uri) = params.and_then(|p| p.get("uri")).and_then(|u| u.as_str()) {
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
