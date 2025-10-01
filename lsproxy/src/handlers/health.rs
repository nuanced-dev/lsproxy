use crate::api_types::{HealthResponse, SupportedLanguages};
use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
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
    let mut languages = HashMap::new();

    // Check which language containers are currently running
    for lang in [
        SupportedLanguages::Python,
        SupportedLanguages::TypeScriptJavaScript,
        SupportedLanguages::Rust,
        SupportedLanguages::CPP,
        SupportedLanguages::CSharp,
        SupportedLanguages::Java,
        SupportedLanguages::Golang,
        SupportedLanguages::PHP,
        SupportedLanguages::Ruby,
        SupportedLanguages::RubySorbet,
    ] {
        let container_available = data.orchestrator.get_container(&lang).await.is_some();
        languages.insert(lang, container_available);
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
    })
}
