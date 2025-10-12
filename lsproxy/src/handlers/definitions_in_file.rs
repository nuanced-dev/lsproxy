use crate::api_types::{ErrorResponse, FileSymbolsRequest, Symbol};
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::{error, info};

/// Get all symbol definitions in a file
#[utoipa::path(
    get,
    path = "/symbol/definitions-in-file",
    tag = "symbol",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = Vec<Symbol>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn definitions_in_file(
    data: Data<AppState>,
    info: Query<FileSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received definitions in file request for file: {}",
        info.file_path
    );

    // Get container client for this file's language
    let client = match container_proxy::get_client_for_file(
        &data.orchestrator,
        &data.workspace_path,
        &info.file_path,
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
    match client.definitions_in_file(&info.into_inner()).await {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Container request failed: {}", e),
            })
        }
    }
}
