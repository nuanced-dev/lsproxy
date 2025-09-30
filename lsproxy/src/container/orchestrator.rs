use super::{ContainerInfo, ContainerOrchestrator, OrchestratorError};
use crate::api_types::SupportedLanguages;
use bollard::container::{Config, CreateContainerOptions, LogsOptions};
use bollard::models::{HostConfig, PortBinding};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::net::TcpListener;
use std::time::Duration;

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
        workspace_path: &str,
    ) -> Result<ContainerInfo, OrchestratorError> {
        // Check if container already exists for this language
        if let Some(existing) = self.get_container(&language).await {
            log::info!(
                "Container already exists for {:?}: {}",
                language,
                existing.container_id
            );
            return Ok(existing);
        }

        let image_name = Self::image_name_for_language(&language);
        let container_name = format!(
            "lsproxy-{}-{}",
            Self::language_slug(&language),
            uuid::Uuid::new_v4()
        );

        // Get configuration from environment
        let host =
            std::env::var("LSPROXY_CONTAINER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let memory_limit_mb: i64 = std::env::var("LSPROXY_CONTAINER_MEMORY_MB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2048); // Default 2GB

        // Reserve a port by keeping the listener alive until container is created
        let bind_addr = format!("{}:0", host);
        let port_listener = TcpListener::bind(&bind_addr)?;
        let port = port_listener.local_addr()?.port();

        // Configure container with read-write workspace mount
        let host_config = HostConfig {
            binds: Some(vec![format!("{}:/workspace:rw", workspace_path)]),
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
            ..Default::default()
        };

        // Pass through RUST_LOG from parent process, or default to "info"
        let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let mut env = vec![format!("RUST_LOG={}", rust_log)];

        // Add language-specific environment variables
        env.extend(Self::language_specific_env(&language));

        let config = Config {
            image: Some(image_name.clone()),
            env: Some(env),
            host_config: Some(host_config),
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

        let endpoint = format!("http://{}:{}", host, port);

        let info = ContainerInfo {
            container_id: container_id.clone(),
            image_name,
            port,
            endpoint: endpoint.clone(),
        };

        // Store container info
        self.store_container(language.clone(), info.clone()).await;

        // Wait for container to be healthy (optional - controlled by env var)
        // This will be used once Phase 4 (HTTP wrapper) is implemented
        if std::env::var("LSPROXY_ENABLE_HEALTH_CHECK").is_ok() {
            self.check_container_health(&info).await?;
        } else {
            log::debug!("Skipping health check (LSPROXY_ENABLE_HEALTH_CHECK not set)");
            // Give container a moment to start
            tokio::time::sleep(Duration::from_millis(500)).await;
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
    pub async fn check_container_health(&self, info: &ContainerInfo) -> Result<(), OrchestratorError> {
        let health_url = format!("{}/health", info.endpoint);
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();
        let client = reqwest::Client::new();

        log::info!("Checking health of container {} at {}", info.container_id, health_url);

        while start.elapsed() < timeout {
            match client.get(&health_url).timeout(Duration::from_secs(2)).send().await {
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
            SupportedLanguages::Ruby => "lsproxy-ruby:latest".to_string(),
            SupportedLanguages::RubySorbet => "lsproxy-ruby-sorbet:latest".to_string(),
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
            SupportedLanguages::Ruby => "ruby",
            SupportedLanguages::RubySorbet => "ruby-sorbet",
            SupportedLanguages::Rust => "rust",
            SupportedLanguages::CPP => "clangd",
            SupportedLanguages::Java => "java",
            SupportedLanguages::PHP => "php",
            SupportedLanguages::CSharp => "csharp",
        }
        .to_string()
    }

    /// Get language-specific environment variables
    fn language_specific_env(language: &SupportedLanguages) -> Vec<String> {
        match language {
            SupportedLanguages::Golang => vec!["LSP_COMMAND=gopls".to_string()],
            SupportedLanguages::Python => vec!["LSP_COMMAND=jedi-language-server".to_string()],
            SupportedLanguages::TypeScriptJavaScript => {
                vec!["LSP_COMMAND=typescript-language-server --stdio".to_string()]
            }
            SupportedLanguages::Ruby => vec!["LSP_COMMAND=ruby-lsp --use-launcher".to_string()],
            SupportedLanguages::RubySorbet => vec!["LSP_COMMAND=srb tc --lsp".to_string()],
            SupportedLanguages::Rust => vec!["LSP_COMMAND=rust-analyzer".to_string()],
            SupportedLanguages::CPP => vec!["LSP_COMMAND=clangd".to_string()],
            SupportedLanguages::Java => vec!["LSP_COMMAND=jdtls".to_string()],
            SupportedLanguages::PHP => vec!["LSP_COMMAND=phpactor language-server".to_string()],
            SupportedLanguages::CSharp => vec!["LSP_COMMAND=csharp-ls".to_string()],
        }
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
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::RubySorbet),
            "lsproxy-ruby-sorbet:latest"
        );
        assert_eq!(
            ContainerOrchestrator::image_name_for_language(&SupportedLanguages::TypeScriptJavaScript),
            "lsproxy-typescript:latest"
        );
    }

    #[test]
    fn test_language_slug() {
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::RubySorbet),
            "ruby-sorbet"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::Golang),
            "golang"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::CSharp),
            "csharp"
        );
    }

    #[test]
    fn test_language_specific_env() {
        let env = ContainerOrchestrator::language_specific_env(&SupportedLanguages::Golang);
        assert_eq!(env.len(), 1);
        assert!(env[0].contains("gopls"));

        let env = ContainerOrchestrator::language_specific_env(&SupportedLanguages::Ruby);
        assert!(env[0].contains("ruby-lsp"));
    }

    // Integration tests - these require Docker to be running
    // Run with: cargo test --test container_tests -- --ignored

    #[tokio::test]
    #[ignore] // Requires Docker
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
    #[ignore] // Requires Docker
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
    #[ignore] // Requires Docker
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
            .spawn_container(SupportedLanguages::Python, "/tmp")
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
