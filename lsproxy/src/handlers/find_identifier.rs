use crate::api_types::{ErrorResponse, FindIdentifierRequest, IdentifierResponse};
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::{error, info};

/// Find an identifier by name
#[utoipa::path(
    get,
    path = "/symbol/find-identifier",
    tag = "symbol",
    params(FindIdentifierRequest),
    responses(
        (status = 200, description = "Identifier found successfully", body = IdentifierResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_identifier(
    data: Data<AppState>,
    info: Query<FindIdentifierRequest>,
) -> HttpResponse {
    info!(
        "Received find identifier request for file: {}, name: {}",
        info.path, info.name
    );

    // Get container client for this file's language
    let client = match container_proxy::get_client_for_file(
        &data.orchestrator,
        &data.workspace_path,
        &info.path,
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
    match client.find_identifier(&info.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Container request failed: {}", e),
            })
        }
    }
}
