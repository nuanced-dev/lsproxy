use actix_web::web::Data;
use actix_web::HttpResponse;
use log::error;

use crate::handlers::error::IntoHttpResponse;
use crate::AppState;

/// Get a list of all files in the workspace
///
/// Returns an array of file paths for all files in the current workspace.
///
/// This is a convenience endpoint that does not use the underlying Language Servers directly, but it does apply the same filtering.
#[utoipa::path(
    get,
    path = "/workspace/list-files",
    tag = "workspace",
    responses(
        (status = 200, description = "Workspace files retrieved successfully", body = Vec<String>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_files(data: Data<AppState>) -> HttpResponse {
    let files = data.manager.list_files().await;
    match files {
        Ok(files) => HttpResponse::Ok().json(files),
        Err(e) => {
            error!("Failed to get workspace files: {}", e);
            e.into_http_response()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_workspace_files() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let response = list_files(state).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let mut workspace_files_response: Vec<String> = serde_json::from_slice(&bytes).unwrap();

        let mut expected = [
            "__init__.py",
            "decorators.py",
            "graph.py",
            "main.py",
            "search.py",
        ];
        expected.sort();
        workspace_files_response.sort();
        assert_eq!(workspace_files_response, expected);
        Ok(())
    }
}
