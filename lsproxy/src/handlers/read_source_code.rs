use crate::api_types::{ErrorResponse, ReadSourceCodeRequest};
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct ReadSourceResponse {
    content: String,
}

/// Read source code from a file
#[utoipa::path(
    post,
    path = "/workspace/read-source-code",
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

    // Build full path
    let file_path = PathBuf::from(&data.workspace_path).join(&info.path);

    // Security check: ensure path is within workspace
    let canonical_workspace = match std::fs::canonicalize(&data.workspace_path) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to canonicalize workspace path: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Invalid workspace path".to_string(),
            });
        }
    };

    let canonical_file = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(e) => {
            error!("File not found: {}", e);
            return HttpResponse::NotFound().json(ErrorResponse {
                error: format!("File not found: {}", info.path),
            });
        }
    };

    if !canonical_file.starts_with(&canonical_workspace) {
        error!("Path traversal attempt: {}", info.path);
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid file path".to_string(),
        });
    }

    // Read the file content
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            // If range is specified, return only that portion
            if let Some(range) = &info.range {
                let lines: Vec<&str> = content.lines().collect();
                let start_line = range.start.line as usize;
                let end_line = range.end.line as usize;

                if start_line >= lines.len() {
                    return HttpResponse::BadRequest().json(ErrorResponse {
                        error: "Start line out of range".to_string(),
                    });
                }

                let end_line = end_line.min(lines.len());
                let selected_lines = &lines[start_line..end_line];
                let content = selected_lines.join("\n");

                HttpResponse::Ok().json(ReadSourceResponse { content })
            } else {
                HttpResponse::Ok().json(ReadSourceResponse { content })
            }
        }
        Err(e) => {
            error!("Failed to read file: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to read file: {}", e),
            })
        }
    }
}
