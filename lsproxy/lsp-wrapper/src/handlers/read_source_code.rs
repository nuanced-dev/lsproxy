use crate::api_types::{ErrorResponse, ReadSourceCodeRequest};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, Range as LspRange};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadSourceCodeResponse {
    pub source_code: String,
}

/// Read source code from a file in the workspace
///
/// Returns the contents of the specified file.
#[utoipa::path(
    post,
    path = "/workspace/read-source-code",
    tag = "workspace",
    request_body = ReadSourceCodeRequest,
    responses(
        (status = 200, description = "Source code retrieved successfully", body = ReadSourceCodeResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn read_source_code(
    data: Data<AppState>,
    info: Json<ReadSourceCodeRequest>,
) -> HttpResponse {
    info!("Reading source code from file: {}", info.path);

    let lsp_range = info.range.as_ref().map(|range| {
        LspRange::new(
            LspPosition {
                line: range.start.line,
                character: range.start.character,
            },
            LspPosition {
                line: range.end.line,
                character: range.end.character,
            },
        )
    });

    match data.manager.read_source_code(&info.path, lsp_range).await {
        Ok(source_code) => HttpResponse::Ok().json(ReadSourceCodeResponse { source_code }),
        Err(e) => {
            error!("Failed to read source code: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to read source code: {}", e),
            })
        }
    }
}
