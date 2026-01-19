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

    // Get all languages with tracked health status
    // This includes both successfully spawned containers and those that failed to spawn
    let all_containers_health = data.orchestrator.get_all_containers_health().await;

    let mut languages = HashMap::new();
    for (lang, health_status) in all_containers_health {
        match health_status {
            ContainerHealthStatus::Healthy => {
                languages.insert(lang, true);
            }
            ContainerHealthStatus::Unhealthy => {
                languages.insert(lang, false);
            }
            ContainerHealthStatus::Pending => {
                // Don't include pending languages in the response
            }
        }
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
    })
}
