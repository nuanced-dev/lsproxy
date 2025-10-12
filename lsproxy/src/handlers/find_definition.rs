use crate::api_types::{DefinitionResponse, ErrorResponse, GetDefinitionRequest};
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};

/// Get the definition of a symbol at a specific position in a file
#[utoipa::path(
    post,
    path = "/symbol/find-definition",
    tag = "symbol",
    request_body = GetDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_definition(
    data: Data<AppState>,
    info: Json<GetDefinitionRequest>,
) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.position.line, info.position.position.character
    );

    // Get container client for this file's language
    let client = match container_proxy::get_client_for_file(
        &data.orchestrator,
        &data.workspace_path,
        &info.position.path,
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
    match client.find_definition(&info.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Container request failed: {}", e),
            })
        }
    }
}
