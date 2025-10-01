/// Helper module for routing requests to containerized LSP servers
///
/// This module handles:
/// - Detecting language from file path
/// - Getting or spawning appropriate container
/// - Making HTTP requests to container
/// - Returning responses

use crate::api_types::*;
use crate::container::{ContainerHttpClient, ContainerOrchestrator};
use crate::utils::file_utils::detect_language;
use log::{error, info};
use std::sync::Arc;

/// Get or spawn a container for the given language and return an HTTP client
pub async fn get_container_client(
    orchestrator: &Arc<ContainerOrchestrator>,
    workspace_path: &str,
    language: SupportedLanguages,
) -> Result<ContainerHttpClient, String> {
    // Check if container already exists
    if let Some(container_info) = orchestrator.get_container(&language).await {
        return Ok(ContainerHttpClient::new(&container_info.endpoint));
    }

    // Spawn new container
    info!("Spawning container for {:?}", language);
    match orchestrator.spawn_container(language.clone(), workspace_path).await {
        Ok(container_info) => {
            info!("Container spawned for {:?}: {}", language, container_info.endpoint);
            Ok(ContainerHttpClient::new(&container_info.endpoint))
        }
        Err(e) => {
            error!("Failed to spawn container for {:?}: {}", language, e);
            Err(format!("Failed to spawn container: {}", e))
        }
    }
}

/// Detect language from file path and get/spawn appropriate container client
pub async fn get_client_for_file(
    orchestrator: &Arc<ContainerOrchestrator>,
    workspace_path: &str,
    file_path: &str,
) -> Result<ContainerHttpClient, String> {
    let language = detect_language(file_path)
        .map_err(|e| format!("Failed to detect language for {}: {}", file_path, e))?;

    get_container_client(orchestrator, workspace_path, language).await
}
