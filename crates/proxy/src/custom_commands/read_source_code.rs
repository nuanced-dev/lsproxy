use actix_web::HttpResponse;
use common::api_types::{get_mount_dir, JsonRpcRequest, JsonRpcResponse, ReadSourceCodeRequest};
use log::{error, info};
use serde::Serialize;

#[derive(Serialize)]
struct ReadSourceResponse {
    source_code: String,
}

pub async fn handle(request: JsonRpcRequest) -> HttpResponse {
    let req_id = request.id.clone();

    let params = match request.params.clone() {
        Some(p) => p,
        None => {
            error!("Missing parameters for readSourceCode");
            let error = JsonRpcResponse::new_error(req_id, -32602, "Missing params");
            return HttpResponse::BadRequest().json(error);
        }
    };

    let info: ReadSourceCodeRequest = match serde_json::from_value(params) {
        Ok(info) => info,
        Err(e) => {
            error!("Invalid parameters for readSourceCode: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32602, format!("Invalid params: {}", e));
            return HttpResponse::BadRequest().json(error);
        }
    };

    info!("Received read source code request for file: {}", info.path);

    // Build full path
    let workspace_path = get_mount_dir();
    let file_path = workspace_path.join(&info.path);

    // Security check: ensure path is within workspace
    let canonical_workspace = match std::fs::canonicalize(&workspace_path) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to canonicalize workspace path: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32603, "Invalid workspace path".to_string());
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let canonical_file = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(e) => {
            error!("File not found: {}", e);
            let error = JsonRpcResponse::new_error(
                req_id,
                -32602,
                format!("File not found: {}", info.path),
            );
            return HttpResponse::NotFound().json(error);
        }
    };

    if !canonical_file.starts_with(&canonical_workspace) {
        error!("Path traversal attempt: {}", info.path);
        let error = JsonRpcResponse::new_error(req_id, -32602, "Invalid file path".to_string());
        return HttpResponse::BadRequest().json(error);
    }

    // Read the file content
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            // If range is specified, return only that portion
            let source_code = if let Some(range) = &info.range {
                let lines: Vec<&str> = content.lines().collect();
                let start_line = range.start.line as usize;
                let end_line = range.end.line as usize;

                if start_line >= lines.len() {
                    let error = JsonRpcResponse::new_error(
                        req_id,
                        -32602,
                        "Start line out of range".to_string(),
                    );
                    return HttpResponse::BadRequest().json(error);
                }

                let end_line = end_line.min(lines.len());
                let selected_lines = &lines[start_line..end_line];
                selected_lines.join("\n")
            } else {
                content
            };

            let response = ReadSourceResponse { source_code };
            let json_rpc_response = JsonRpcResponse::new_result(req_id, response);

            HttpResponse::Ok().json(json_rpc_response)
        }
        Err(e) => {
            error!("Failed to read file: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32603, format!("Failed to read file: {}", e));
            HttpResponse::InternalServerError().json(error)
        }
    }
}
