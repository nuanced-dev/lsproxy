use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use ignore::WalkBuilder;
use log::{error, info};

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

    let mut files = Vec::new();
    let workspace_path = &data.workspace_path;

    // Walk the workspace directory directly (no container calls needed)
    for result in WalkBuilder::new(workspace_path)
        .hidden(false)      // Skip hidden files
        .git_ignore(false)  // Don't filter by gitignore - list all workspace files
        .git_exclude(false) // Don't use git exclude rules
        .build()
    {
        match result {
            Ok(entry) => {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    if let Ok(relative) = entry.path().strip_prefix(workspace_path) {
                        if let Some(rel_str) = relative.to_str() {
                            files.push(rel_str.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error walking workspace: {}", e);
            }
        }
    }

    files.sort();
    files.dedup();

    HttpResponse::Ok().json(files)
}
