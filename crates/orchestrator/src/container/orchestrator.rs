use super::{ContainerInfo, ContainerOrchestrator, OrchestratorError};
use bollard::container::{Config, CreateContainerOptions, LogsOptions};
use bollard::models::{HostConfig, PortBinding};
use futures_util::stream::StreamExt;
use lsproxy_common::api_types::SupportedLanguages;
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

impl ContainerOrchestrator {
    /// Spawn a container for a specific language
    /// NOTE: This method should be called with external synchronization (e.g., holding a lock)
    /// to prevent port allocation races when spawning multiple containers concurrently.
    ///
    /// # Error Cases
    /// - `OrchestratorError::Docker`: Docker daemon not accessible or image doesn't exist
    /// - `OrchestratorError::Io`: Port binding failure or workspace path invalid
    /// - `OrchestratorError::Docker`: Container fails to start (resource limits, LSP crash)
    pub async fn spawn_container(
        &self,
        language: SupportedLanguages,
    ) -> Result<ContainerInfo, OrchestratorError> {
        // Quick check without holding lock for long - fast path for existing containers
        {
            let containers_guard = self.containers.lock().await;
            if let Some(existing) = containers_guard.get(&language).cloned() {
                log::info!(
                    "Container already exists for {:?}: {}",
                    language,
                    existing.container_id
                );
                return Ok(existing);
            }
        }

        // Get or create a per-language spawning lock to prevent duplicate spawns of same language
        // while allowing concurrent spawns of different languages
        let language_lock = {
            let mut spawning_locks = self.spawning_locks.lock().await;
            spawning_locks
                .entry(language.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        // Acquire the per-language lock for the entire spawn operation
        let _language_guard = language_lock.lock().await;

        // Double-check container doesn't exist (another request might have created it while we waited for the lock)
        {
            let containers_guard = self.containers.lock().await;
            if let Some(existing) = containers_guard.get(&language).cloned() {
                log::info!(
                    "Container already exists for {:?}: {} (created while waiting for spawn lock)",
                    language,
                    existing.container_id
                );
                return Ok(existing);
            }
        }

        // Ensure wrapper container is running before spawning language containers
        let wrapper_container_id = self.ensure_wrapper_container().await?;
        log::debug!("Using wrapper container: {}", wrapper_container_id);

        let image_name = Self::image_name_for_language(&language);
        let container_name = format!(
            "lsproxy-{}-{}",
            Self::language_slug(&language),
            uuid::Uuid::new_v4()
        );

        // Get configuration from environment
        let host =
            std::env::var("LSPROXY_CONTAINER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let memory_limit_mb: i64 = std::env::var("LSPROXY_MAX_MEMORY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20480); // Default 20GB (in MB)

        // Reserve a port by keeping the listener alive until container is created
        let bind_addr = format!("{}:0", host);
        let port_listener = TcpListener::bind(&bind_addr)?;
        let port = port_listener.local_addr()?.port();

        // Configure container with read-write workspace mount
        // When running in Docker, we need the original host path, not our container's mount point
        // Docker interprets mount sources from the host's perspective when using the Docker socket
        //
        // Auto-detect host workspace path by inspecting our own container's mounts
        // Fall back to HOST_WORKSPACE_PATH env var (for backwards compatibility)
        // Error if neither method works
        let mount_source = if let Some(host_path) = self.get_host_workspace_path().await {
            host_path
        } else if let Ok(env_path) = std::env::var("HOST_WORKSPACE_PATH") {
            log::info!("Using HOST_WORKSPACE_PATH from environment: {}", env_path);
            env_path
        } else {
            return Err(OrchestratorError::Configuration(
                "Cannot determine host workspace path: auto-detection failed and HOST_WORKSPACE_PATH not set".to_string()
            ));
        };

        let host_config = HostConfig {
            binds: Some(vec![format!("{}:/mnt/workspace:rw", mount_source)]),
            port_bindings: Some({
                let mut ports = HashMap::new();
                ports.insert(
                    "8080/tcp".to_string(),
                    Some(vec![PortBinding {
                        host_ip: Some(host.clone()),
                        host_port: Some(port.to_string()),
                    }]),
                );
                ports
            }),
            memory: Some(memory_limit_mb * 1024 * 1024), // Convert MB to bytes
            // Mount wrapper binary and ast-grep configs from wrapper container
            volumes_from: Some(vec![wrapper_container_id.clone()]),
            ..Default::default()
        };

        // Pass through all environment variables from parent process
        // This ensures LSP containers inherit configuration like RUST_LOG, custom settings, etc.
        let env: Vec<String> = std::env::vars()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();

        // Label containers with parent ID for watchdog cleanup
        let mut labels = HashMap::new();
        labels.insert("lsproxy.role".to_string(), "language-server".to_string());
        if let Some(parent_id) = ContainerOrchestrator::get_own_container_id() {
            labels.insert("lsproxy.parent".to_string(), parent_id);
        }

        let config = Config {
            image: Some(image_name.clone()),
            env: Some(env),
            host_config: Some(host_config),
            labels: Some(labels),
            exposed_ports: Some({
                let mut ports = HashMap::new();
                ports.insert("8080/tcp".to_string(), HashMap::new());
                ports
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create the container
        log::info!("Creating container {} for {:?}", container_name, language);
        let container = self.docker.create_container(Some(options), config).await?;
        let container_id = container.id;

        // Start the container
        log::info!("Starting container {} for {:?}", container_id, language);
        self.docker
            .start_container::<String>(&container_id, None)
            .await?;

        // Now that container is starting and will bind to the port, we can release our reservation
        drop(port_listener);

        // Build endpoint URL for HTTP requests
        // For Docker-outside-of-Docker sibling container communication, we need to use the container's
        // IP address on the Docker network instead of going through host port mappings.
        // Inspect the container to get its IP address.
        let container_info = self.docker.inspect_container(&container_id, None).await?;
        let request_host = if let Some(network_settings) = container_info.network_settings {
            if let Some(networks) = network_settings.networks {
                // Try to find the bridge network (default) or any network with an IP
                networks
                    .get("bridge")
                    .or_else(|| networks.values().next())
                    .and_then(|network| network.ip_address.clone())
                    .filter(|ip| !ip.is_empty())
                    .unwrap_or_else(|| {
                        log::warn!(
                            "Could not find container IP, falling back to host.docker.internal"
                        );
                        "host.docker.internal".to_string()
                    })
            } else {
                log::warn!("No networks found, falling back to host.docker.internal");
                "host.docker.internal".to_string()
            }
        } else {
            log::warn!("No network settings found, falling back to host.docker.internal");
            "host.docker.internal".to_string()
        };

        // For container-to-container communication on Docker network, use port 8080 (internal port)
        // not the host-mapped port
        let internal_port = 8080;
        let endpoint = format!("http://{}:{}", request_host, internal_port);

        let info = ContainerInfo {
            container_id: container_id.clone(),
            image_name,
            port,
            endpoint: endpoint.clone(),
        };

        // Wait for container to be healthy before registering it
        // IMPORTANT: Do this BEFORE inserting into the map to avoid registering unhealthy containers
        self.check_container_health(&info).await?;

        // Store container info after health check passes
        // Only insert after health check passes to ensure we don't register unhealthy containers
        {
            let mut containers_guard = self.containers.lock().await;
            containers_guard.insert(language.clone(), info.clone());
        }

        log::info!(
            "Successfully spawned container {} for {:?} at {}",
            container_id,
            language,
            endpoint
        );

        Ok(info)
    }

    /// Check if a container is healthy by polling its /health endpoint
    ///
    /// This requires the HTTP wrapper (Phase 4) to be implemented in the container.
    /// The health check simply verifies the wrapper is responding (simple mode).
    ///
    /// # Arguments
    /// * `info` - Container information including endpoint
    ///
    /// # Returns
    /// * `Ok(())` if container responds with healthy status
    /// * `Err(OrchestratorError::HealthCheck)` if health check fails or times out
    pub async fn check_container_health(
        &self,
        info: &ContainerInfo,
    ) -> Result<(), OrchestratorError> {
        let health_url = format!("{}/health", info.endpoint);
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();
        let client = reqwest::Client::new();

        log::info!(
            "Checking health of container {} at {}",
            info.container_id,
            health_url
        );

        while start.elapsed() < timeout {
            match client
                .get(&health_url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    log::info!("Container {} is healthy", info.container_id);
                    return Ok(());
                }
                Ok(response) => {
                    log::debug!(
                        "Container {} health check returned status: {}",
                        info.container_id,
                        response.status()
                    );
                }
                Err(e) => {
                    log::debug!("Health check attempt failed: {}", e);
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Health check failed - get container logs for debugging
        let logs = self.get_container_logs(&info.container_id, 50).await;
        let error_msg = format!(
            "Container {} health check timeout after {:?}. Recent logs:\n{}",
            info.container_id,
            timeout,
            logs.unwrap_or_else(|| "Could not retrieve logs".to_string())
        );

        Err(OrchestratorError::HealthCheck(error_msg))
    }

    /// Get recent logs from a container for debugging
    ///
    /// # Arguments
    /// * `container_id` - The container ID
    /// * `tail` - Number of lines to retrieve
    ///
    /// # Returns
    /// * `Some(String)` containing the logs, or `None` if logs couldn't be retrieved
    async fn get_container_logs(&self, container_id: &str, tail: usize) -> Option<String> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));
        let mut logs = String::new();

        while let Some(Ok(log)) = stream.next().await {
            logs.push_str(&log.to_string());
        }

        if logs.is_empty() {
            None
        } else {
            Some(logs)
        }
    }

    /// Get the Docker image name for a language
    fn image_name_for_language(language: &SupportedLanguages) -> String {
        match language {
            SupportedLanguages::Golang => "lsproxy-golang:latest".to_string(),
            SupportedLanguages::Python => "lsproxy-python:latest".to_string(),
            SupportedLanguages::TypeScriptJavaScript => "lsproxy-typescript:latest".to_string(),
            SupportedLanguages::Ruby3_4_7 => "lsproxy-ruby-3.4.7:latest".to_string(),
            SupportedLanguages::Ruby3_4_6 => "lsproxy-ruby-3.4.6:latest".to_string(),
            SupportedLanguages::Ruby3_4_5 => "lsproxy-ruby-3.4.5:latest".to_string(),
            SupportedLanguages::Ruby3_4_4 => "lsproxy-ruby-3.4.4:latest".to_string(),
            SupportedLanguages::Ruby3_4_3 => "lsproxy-ruby-3.4.3:latest".to_string(),
            SupportedLanguages::Ruby3_4_2 => "lsproxy-ruby-3.4.2:latest".to_string(),
            SupportedLanguages::Ruby3_4_1 => "lsproxy-ruby-3.4.1:latest".to_string(),
            SupportedLanguages::Ruby3_4_0 => "lsproxy-ruby-3.4.0:latest".to_string(),
            SupportedLanguages::Ruby3_3_6 => "lsproxy-ruby-3.3.6:latest".to_string(),
            SupportedLanguages::Ruby3_3_5 => "lsproxy-ruby-3.3.5:latest".to_string(),
            SupportedLanguages::Ruby3_2_6 => "lsproxy-ruby-3.2.6:latest".to_string(),
            SupportedLanguages::Ruby3_2_2 => "lsproxy-ruby-3.2.2:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_7 => "lsproxy-ruby-sorbet-3.4.7:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_6 => "lsproxy-ruby-sorbet-3.4.6:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_5 => "lsproxy-ruby-sorbet-3.4.5:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_4 => "lsproxy-ruby-sorbet-3.4.4:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_3 => "lsproxy-ruby-sorbet-3.4.3:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_2 => "lsproxy-ruby-sorbet-3.4.2:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_1 => "lsproxy-ruby-sorbet-3.4.1:latest".to_string(),
            SupportedLanguages::RubySorbet3_4_0 => "lsproxy-ruby-sorbet-3.4.0:latest".to_string(),
            SupportedLanguages::RubySorbet3_3_6 => "lsproxy-ruby-sorbet-3.3.6:latest".to_string(),
            SupportedLanguages::RubySorbet3_3_5 => "lsproxy-ruby-sorbet-3.3.5:latest".to_string(),
            SupportedLanguages::RubySorbet3_2_6 => "lsproxy-ruby-sorbet-3.2.6:latest".to_string(),
            SupportedLanguages::RubySorbet3_2_2 => "lsproxy-ruby-sorbet-3.2.2:latest".to_string(),
            SupportedLanguages::Rust => "lsproxy-rust:latest".to_string(),
            SupportedLanguages::CPP => "lsproxy-clangd:latest".to_string(),
            SupportedLanguages::Java => "lsproxy-java:latest".to_string(),
            SupportedLanguages::PHP => "lsproxy-php:latest".to_string(),
            SupportedLanguages::CSharp => "lsproxy-csharp:latest".to_string(),
        }
    }

    /// Get a URL-safe slug for a language
    fn language_slug(language: &SupportedLanguages) -> String {
        match language {
            SupportedLanguages::Golang => "golang",
            SupportedLanguages::Python => "python",
            SupportedLanguages::TypeScriptJavaScript => "typescript",
            SupportedLanguages::Ruby3_4_7 => "ruby-3.4.7",
            SupportedLanguages::Ruby3_4_6 => "ruby-3.4.6",
            SupportedLanguages::Ruby3_4_5 => "ruby-3.4.5",
            SupportedLanguages::Ruby3_4_4 => "ruby-3.4.4",
            SupportedLanguages::Ruby3_4_3 => "ruby-3.4.3",
            SupportedLanguages::Ruby3_4_2 => "ruby-3.4.2",
            SupportedLanguages::Ruby3_4_1 => "ruby-3.4.1",
            SupportedLanguages::Ruby3_4_0 => "ruby-3.4.0",
            SupportedLanguages::Ruby3_3_6 => "ruby-3.3.6",
            SupportedLanguages::Ruby3_3_5 => "ruby-3.3.5",
            SupportedLanguages::Ruby3_2_6 => "ruby-3.2.6",
            SupportedLanguages::Ruby3_2_2 => "ruby-3.2.2",
            SupportedLanguages::RubySorbet3_4_7 => "ruby-sorbet-3.4.7",
            SupportedLanguages::RubySorbet3_4_6 => "ruby-sorbet-3.4.6",
            SupportedLanguages::RubySorbet3_4_5 => "ruby-sorbet-3.4.5",
            SupportedLanguages::RubySorbet3_4_4 => "ruby-sorbet-3.4.4",
            SupportedLanguages::RubySorbet3_4_3 => "ruby-sorbet-3.4.3",
            SupportedLanguages::RubySorbet3_4_2 => "ruby-sorbet-3.4.2",
            SupportedLanguages::RubySorbet3_4_1 => "ruby-sorbet-3.4.1",
            SupportedLanguages::RubySorbet3_4_0 => "ruby-sorbet-3.4.0",
            SupportedLanguages::RubySorbet3_3_6 => "ruby-sorbet-3.3.6",
            SupportedLanguages::RubySorbet3_3_5 => "ruby-sorbet-3.3.5",
            SupportedLanguages::RubySorbet3_2_6 => "ruby-sorbet-3.2.6",
            SupportedLanguages::RubySorbet3_2_2 => "ruby-sorbet-3.2.2",
            SupportedLanguages::Rust => "rust",
            SupportedLanguages::CPP => "clangd",
            SupportedLanguages::Java => "java",
            SupportedLanguages::PHP => "php",
            SupportedLanguages::CSharp => "csharp",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests - these don't require Docker

    #[test]
    fn test_image_name_for_language() {
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Golang),
            "lsproxy-golang:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Python),
            "lsproxy-python:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(
                &SupportedLanguages::TypeScriptJavaScript
            ),
            "lsproxy-typescript:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Ruby3_4_4),
            "lsproxy-ruby-3.4.4:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Ruby3_3_6),
            "lsproxy-ruby-3.3.6:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::RubySorbet3_4_4),
            "lsproxy-ruby-sorbet-3.4.4:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Rust),
            "lsproxy-rust:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::CPP),
            "lsproxy-clangd:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::Java),
            "lsproxy-java:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::PHP),
            "lsproxy-php:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::CSharp),
            "lsproxy-csharp:latest"
        );
    }

    #[test]
    fn test_language_slug() {
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Golang),
            "golang"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Python),
            "python"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::TypeScriptJavaScript),
            "typescript"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Ruby3_4_4),
            "ruby-3.4.4"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Ruby3_3_6),
            "ruby-3.3.6"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::RubySorbet3_4_4),
            "ruby-sorbet-3.4.4"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Rust),
            "rust"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::CPP),
            "clangd"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Java),
            "java"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::PHP),
            "php"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::CSharp),
            "csharp"
        );
    }

    // Integration tests - these require Docker to be running
    // Run with: cargo test --test container_tests -- --ignored

    #[tokio::test]
    async fn test_store_and_get_container() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        // Initially no containers
        assert!(orchestrator
            .get_container(&SupportedLanguages::Python)
            .await
            .is_none());

        // Store a container
        let info = ContainerInfo {
            container_id: "test-123".to_string(),
            image_name: "lsproxy-python:latest".to_string(),
            port: 8080,
            endpoint: "http://0.0.0.0:8080".to_string(),
        };

        orchestrator
            .store_container(SupportedLanguages::Python, info.clone())
            .await;

        // Should be able to retrieve it
        let retrieved = orchestrator
            .get_container(&SupportedLanguages::Python)
            .await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().container_id, "test-123");

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_container_from_map() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        let info = ContainerInfo {
            container_id: "test-456".to_string(),
            image_name: "lsproxy-golang:latest".to_string(),
            port: 8081,
            endpoint: "http://0.0.0.0:8081".to_string(),
        };

        orchestrator
            .store_container(SupportedLanguages::Golang, info)
            .await;
        assert!(orchestrator
            .get_container(&SupportedLanguages::Golang)
            .await
            .is_some());

        // Remove it
        let removed = orchestrator
            .remove_container(&SupportedLanguages::Golang)
            .await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().container_id, "test-456");

        // Should be gone now
        assert!(orchestrator
            .get_container(&SupportedLanguages::Golang)
            .await
            .is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_all_containers() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        // Start empty
        assert_eq!(orchestrator.all_containers().await.len(), 0);

        // Add two containers
        let info1 = ContainerInfo {
            container_id: "test-1".to_string(),
            image_name: "lsproxy-python:latest".to_string(),
            port: 8080,
            endpoint: "http://0.0.0.0:8080".to_string(),
        };
        let info2 = ContainerInfo {
            container_id: "test-2".to_string(),
            image_name: "lsproxy-golang:latest".to_string(),
            port: 8081,
            endpoint: "http://0.0.0.0:8081".to_string(),
        };

        orchestrator
            .store_container(SupportedLanguages::Python, info1)
            .await;
        orchestrator
            .store_container(SupportedLanguages::Golang, info2)
            .await;

        let all = orchestrator.all_containers().await;
        assert_eq!(all.len(), 2);

        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires Docker and images to be built
    async fn test_spawn_container_returns_existing() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        // Pre-populate with a "container"
        let existing_info = ContainerInfo {
            container_id: "existing-123".to_string(),
            image_name: "lsproxy-python:latest".to_string(),
            port: 9000,
            endpoint: "http://0.0.0.0:9000".to_string(),
        };

        orchestrator
            .store_container(SupportedLanguages::Python, existing_info.clone())
            .await;

        // Try to spawn - should return existing
        let result = orchestrator
            .spawn_container(SupportedLanguages::Python)
            .await?;
        assert_eq!(result.container_id, "existing-123");
        assert_eq!(result.port, 9000);

        Ok(())
    }

    // Note: Full spawn_container test would require:
    // 1. Docker images to be built (lsproxy-golang:latest, etc.)
    // 2. Valid workspace path
    // 3. Cleanup of created containers
    //
    // This should be done in a separate integration test suite that:
    // - Builds a minimal test image
    // - Tests the full lifecycle
    // - Ensures cleanup even on failure
}
