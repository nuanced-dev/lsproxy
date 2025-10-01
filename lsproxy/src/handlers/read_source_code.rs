use crate::api_types::{ErrorResponse, ReadSourceCodeRequest};
use crate::handlers::container_proxy;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use serde::Serialize;

#[derive(Serialize)]
struct ReadSourceResponse {
    content: String,
}

/// Read source code from a file
#[utoipa::path(
    post,
    path = "/file/read-source",
    tag = "file",
    request_body = ReadSourceCodeRequest,
    responses(
        (status = 200, description = "Source code retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn read_source_code(
    data: Data<AppState>,
    info: Json<ReadSourceCodeRequest>,
) -> HttpResponse {
    info!(
        "Received read source code request for file: {}",
        info.path
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
    match client.read_source(&info.into_inner()).await {
        Ok(content) => HttpResponse::Ok().json(ReadSourceResponse { content }),
        Err(e) => {
            error!("Container request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Container request failed: {}", e),
            })
        }
    }
}
