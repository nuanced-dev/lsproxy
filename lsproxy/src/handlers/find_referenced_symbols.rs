use crate::api_types::{ErrorResponse, GetReferencedSymbolsRequest, ReferencedSymbolsResponse};
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};

/// Find all symbols referenced within a given symbol
#[utoipa::path(
    post,
    path = "/symbol/find-referenced-symbols",
    tag = "symbol",
    request_body = GetReferencedSymbolsRequest,
    responses(
        (status = 200, description = "Referenced symbols retrieved successfully", body = ReferencedSymbolsResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_referenced_symbols(
    data: Data<AppState>,
    info: Json<GetReferencedSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received find referenced symbols request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    // Get container client for this file's language
    let client = match container_proxy::get_client_for_file(
        &data.orchestrator,
        &data.workspace_path,
        &info.identifier_position.path,
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to get container client: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get container client: {}", e),
            });
        }
    };

    // Forward request to container
    match client.find_referenced_symbols(&info.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Container request failed: {}", e),
            })
        }
    }
}
