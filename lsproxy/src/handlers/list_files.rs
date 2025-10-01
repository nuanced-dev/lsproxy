use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use log::{error, info};
use serde::Serialize;

#[derive(Serialize)]
struct ListFilesResponse {
    files: Vec<String>,
}

/// List all files in the workspace
#[utoipa::path(
    get,
    path = "/workspace/list-files",
    tag = "file",
    responses(
        (status = 200, description = "Files listed successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_files(data: Data<AppState>) -> HttpResponse {
    info!("Received list files request");

    // Get all running containers and collect their files
    let all_containers = data.orchestrator.all_containers().await;

    if all_containers.is_empty() {
        // No containers running yet - return empty list
        return HttpResponse::Ok().json(ListFilesResponse { files: vec![] });
    }

    let mut all_files = Vec::new();

    for (_lang, container_info) in all_containers {
        let client = crate::container::ContainerHttpClient::new(&container_info.endpoint);
        match client.list_files().await {
            Ok(files) => all_files.extend(files),
            Err(e) => {
                error!("Failed to list files from container {}: {}", container_info.container_id, e);
            }
        }
    }

    // Deduplicate files
    all_files.sort();
    all_files.dedup();

    HttpResponse::Ok().json(ListFilesResponse { files: all_files })
}
