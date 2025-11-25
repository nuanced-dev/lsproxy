use actix_web::HttpResponse;
use common::api_types::{get_mount_dir, JsonRpcRequest, JsonRpcResponse};
use log::{error, info};

pub async fn handle(request: JsonRpcRequest) -> HttpResponse {
    let req_id = request.id.clone();

    info!("Received list files request");

    // Get workspace path from manager's mount_dir
    let workspace_path = get_mount_dir();

    // Use parallel file walking for performance on large workspaces
    use ignore::WalkBuilder;
    use std::sync::{Arc, Mutex};

    let files = Arc::new(Mutex::new(Vec::new()));
    let workspace_path_arc = Arc::new(workspace_path.clone());

    let walker = WalkBuilder::new(&workspace_path)
        .hidden(false) // Include hidden files (like .ruby-version, .python-version)
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

    let json_rpc_response = JsonRpcResponse::new_result(req_id, files);

    HttpResponse::Ok().json(json_rpc_response)
}
