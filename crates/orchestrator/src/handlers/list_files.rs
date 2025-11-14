use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use ignore::WalkBuilder;
use log::{error, info};
use std::path::Path;
use std::sync::{Arc, Mutex};

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

    let workspace_path = Path::new(&data.workspace_path);

    // Use parallel file walking for performance on large workspaces
    let files = Arc::new(Mutex::new(Vec::new()));
    let workspace_path_arc = Arc::new(workspace_path.to_path_buf());

    let walker = WalkBuilder::new(workspace_path)
        .hidden(true) // Skip hidden files (like .git, .env)
        .git_ignore(false) // Don't filter by gitignore - list all workspace files
        .git_exclude(false) // Don't use git exclude rules
        .build_parallel();

    walker.run(|| {
        let files = Arc::clone(&files);
        let workspace_path = Arc::clone(&workspace_path_arc);

        Box::new(move |result| {
            use ignore::WalkState;

            match result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if let Ok(relative) = entry.path().strip_prefix(workspace_path.as_ref()) {
                            if let Some(rel_str) = relative.to_str() {
                                if let Ok(mut files) = files.lock() {
                                    files.push(rel_str.to_string());
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Error walking workspace: {}", e),
            }
            WalkState::Continue
        })
    });

    let mut files = Arc::try_unwrap(files).unwrap().into_inner().unwrap();

    files.sort();
    files.dedup();

    HttpResponse::Ok().json(files)
}
