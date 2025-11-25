use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use common::api_types::HealthResponse;
use std::collections::HashMap;

use crate::container::ContainerHealthStatus;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get health status of the LSP proxy service
///
/// Returns the service status, version and language server availability
/// Returns 503 Service Unavailable while initialization is in progress
#[utoipa::path(
    get,
    path = "/system/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is healthy and initialized", body = HealthResponse),
        (status = 503, description = "Service is initializing", body = HealthResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn health_check(data: Data<AppState>) -> HttpResponse {
    // Check if initialization is complete
    if !data.is_initialized() {
        return HttpResponse::ServiceUnavailable().json(HealthResponse {
            status: "initializing".to_string(),
            version: VERSION.to_string(),
            languages: HashMap::new(),
        });
    }

    // Get all currently running containers from the orchestrator
    let running_containers = data.orchestrator.all_containers().await;

    let mut languages = HashMap::new();
    for (lang, _info) in running_containers {
        let healthy = match data.orchestrator.get_container_health(&lang).await {
            Some(ContainerHealthStatus::Healthy) => true,
            // Treat pending/unhealthy/unknown as not ready yet
            _ => false,
        };
        languages.insert(lang, healthy);
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
    })
}
