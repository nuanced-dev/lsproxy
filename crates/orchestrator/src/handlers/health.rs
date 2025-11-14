use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use lsproxy_common::api_types::HealthResponse;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get health status of the LSP proxy service
///
/// Returns the service status, version and language server availability
#[utoipa::path(
    get,
    path = "/system/health",
    tag = "system",
    responses(
        (status = 200, description = "Health check successful", body = HealthResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn health_check(data: Data<AppState>) -> HttpResponse {
    // Get all currently running containers from the orchestrator
    let running_containers = data.orchestrator.all_containers().await;

    let mut languages = HashMap::new();
    for (lang, _info) in running_containers {
        languages.insert(lang, true);
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
    })
}
