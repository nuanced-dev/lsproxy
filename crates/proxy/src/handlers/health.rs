use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use common::api_types::{HealthResponse, HealthStatus};
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
            language_status: HashMap::new(),
        });
    }

    // Get all languages with tracked health status
    // This includes both successfully spawned containers and those that failed to spawn
    let all_health_statuses = data.orchestrator.get_all_languages_health().await;

    let mut languages = HashMap::new();
    let mut language_status = HashMap::new();
    for (lang, health_status) in all_health_statuses {
        let status = match health_status {
            ContainerHealthStatus::Healthy => HealthStatus::Healthy,
            ContainerHealthStatus::Pending => HealthStatus::Pending,
            ContainerHealthStatus::Unhealthy => HealthStatus::Unhealthy,
        };
        // Backward compatible bool: true only if healthy
        let available = status == HealthStatus::Healthy;
        languages.insert(lang.clone(), available);
        language_status.insert(lang, status);
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
        language_status,
    })
}
