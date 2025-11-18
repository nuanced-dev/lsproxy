use crate::container::ContainerOrchestrator;
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{debug, error, info, warn};
use lsp_types::{
    DeclarationCapability, FoldingRangeProviderCapability, HoverProviderCapability,
    ImplementationProviderCapability, InitializeResult, OneOf, PositionEncodingKind,
    ServerCapabilities, ServerInfo,
};
use lsproxy_common::api_types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;
use std::sync::Arc;
use url::Url;

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
    debug!("LSP request: id={:?}", &lsp_req);

    // Handle lifecycle requests locally
    if let Some(response) = handle_lifecycle_request(&lsp_req) {
        return response;
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
    let file_path = match Url::parse(&document_uri) {
        Ok(url) if url.scheme() == "file" => match url.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => {
                error!("Invalid file URI path: {}", document_uri);
                return HttpResponse::BadRequest().json(JsonRpcResponse::new_error(
                    req_id,
                    -32602,
                    "Invalid params: invalid file URI path",
                ));
            }
        },
        Ok(_) => {
            error!("Non-file URI: {}", document_uri);
            return HttpResponse::BadRequest().json(JsonRpcResponse::new_error(
                req_id,
                -32602,
                "Invalid params: expected file URI",
            ));
        }
        Err(e) => {
            error!("Invalid document URI: {} - {}", document_uri, e);
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

    // Convert request paths from host to container
    let mut converted_request = lsp_req.clone();
    if let Some(ref mut params) = converted_request.params {
        if let Err(e) = convert_json_paths_host_to_container(&data.orchestrator, params).await {
            error!("Failed to convert request paths: {}", e);
            return HttpResponse::InternalServerError().json(JsonRpcResponse::new_error(
                req_id,
                -32603,
                &format!("Path conversion error: {}", e),
            ));
        }
    }

    // Forward request to container
    match client.forward_lsp_request(&converted_request).await {
        Ok(mut response) => {
            info!("Received container response: id={}", response.id);
            debug!("Container response: {:?}", response);

            // Convert response paths from container to host
            if let Some(ref mut result) = response.result {
                if let Err(e) =
                    convert_json_paths_container_to_host(&data.orchestrator, result).await
                {
                    error!("Failed to convert response paths: {}", e);
                    return HttpResponse::InternalServerError().json(JsonRpcResponse::new_error(
                        req_id,
                        -32603,
                        &format!("Path conversion error: {}", e),
                    ));
                }
            }

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

/// Handle lifecycle requests locally
fn handle_lifecycle_request(request: &JsonRpcRequest) -> Option<HttpResponse> {
    let req_id = request.id.clone();
    match request.method.as_str() {
        "initialize" => {
            let mut capabilities = ServerCapabilities::default();
            capabilities.call_hierarchy_provider = Some(true.into());
            capabilities.declaration_provider = Some(DeclarationCapability::Simple(true));
            capabilities.definition_provider = Some(OneOf::Left(true));
            capabilities.document_symbol_provider = Some(OneOf::Left(true));
            capabilities.folding_range_provider =
                Some(FoldingRangeProviderCapability::Simple(true));
            capabilities.hover_provider = Some(HoverProviderCapability::Simple(false));
            capabilities.implementation_provider =
                Some(ImplementationProviderCapability::Simple(true));
            capabilities.inline_value_provider = Some(OneOf::Left(true));
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
            Some(HttpResponse::Ok().json(response))
        }
        "initialized" | "exit" => Some(HttpResponse::Ok().finish()),
        "shutdown" => {
            let response = JsonRpcResponse::new_result(req_id, Value::Null);
            Some(HttpResponse::Ok().json(response))
        }
        dollar_method if dollar_method.starts_with("$/") => {
            if req_id.is_some() {
                let response = JsonRpcResponse::new_error(req_id, -32601, "Method not found");
                Some(HttpResponse::Ok().json(response))
            } else {
                Some(HttpResponse::Ok().finish())
            }
        }
        _ => None,
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

    None
}

/// Recursively convert paths in JSON value from host to container
fn convert_json_paths_host_to_container<'a>(
    orchestrator: &'a Arc<ContainerOrchestrator>,
    value: &'a mut Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + 'a>> {
    Box::pin(async move {
        let host_workspace = orchestrator
            .get_host_workspace_path()
            .await
            .ok_or_else(|| "Failed to get host workspace path".to_string())?;

        convert_json_paths_recursive(value, &host_workspace, "/mnt/workspace");
        Ok(())
    })
}

/// Recursively replace path strings in JSON values
fn convert_json_paths_recursive(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(s) => {
            *s = s.replace(from, to);
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                convert_json_paths_recursive(v, from, to);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                convert_json_paths_recursive(item, from, to);
            }
        }
        _ => {}
    }
}

/// Recursively convert paths in JSON value from container to host
fn convert_json_paths_container_to_host<'a>(
    orchestrator: &'a Arc<ContainerOrchestrator>,
    value: &'a mut Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + 'a>> {
    Box::pin(async move {
        let host_workspace = orchestrator
            .get_host_workspace_path()
            .await
            .ok_or_else(|| "Failed to get host workspace path".to_string())?;

        convert_json_paths_recursive(value, "/mnt/workspace", &host_workspace);
        Ok(())
    })
}
