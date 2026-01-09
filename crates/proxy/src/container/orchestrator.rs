use crate::container::{find_image, language_image};

use super::{ContainerHealthStatus, ContainerInfo, ContainerOrchestrator, OrchestratorError};
use bollard::container::{Config, CreateContainerOptions};
use bollard::models::{HostConfig, PortBinding};
use common::api_types::{LanguageVariant, SupportedLanguages};
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

        // Set health status to Pending at the start of spawn attempt
        self.set_container_health(language.clone(), ContainerHealthStatus::Pending)
            .await;

        // Call implementation and handle health status updates
        let info = match self.spawn_container_impl(language.clone()).await {
            Ok(info) => info,
            Err(e) => {
                self.set_container_health(language, ContainerHealthStatus::Unhealthy)
                    .await;
                return Err(e);
            }
        };

        match self.check_container_health(&info).await {
            Ok(_) => {
                log::info!("{} is now healthy and ready", info.image_name);
                self.set_container_health(language.clone(), ContainerHealthStatus::Healthy)
                    .await;
            }
            Err(e) => {
                log::error!("{} health check failed: {}", info.image_name, e);
                self.set_container_health(language.clone(), ContainerHealthStatus::Unhealthy)
                    .await;
                return Err(OrchestratorError::HealthCheck(format!(
                    "{}: {}",
                    info.image_name, e
                )));
            }
        }

        Ok(info)
    }

    /// Internal implementation of container spawning
    /// All errors are handled by the wrapper `spawn_container` method
    async fn spawn_container_impl(
        &self,
        language: SupportedLanguages,
    ) -> Result<ContainerInfo, OrchestratorError> {
        // Ensure wrapper container is running before spawning language containers
        let wrapper_container_id = self.ensure_wrapper_container().await?;
        log::debug!("Using wrapper container: {}", wrapper_container_id);

        let image_name = language_image(&language);
        let container_name = format!(
            "nuanced-lsp-{}-{}",
            Self::language_slug(&language),
            uuid::Uuid::new_v4()
        );

        // Get configuration from environment
        let host =
            std::env::var("NUANCED_LSP_CONTAINER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let memory_limit_mb: i64 = std::env::var("NUANCED_LSP_MAX_MEMORY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20480); // Default 20GB (in MB)

        // Acquire global port allocation lock to prevent port conflicts across all languages
        // This lock is held briefly - just long enough to allocate a port and start the container
        let _port_lock = self.port_allocation_lock.lock().await;

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
        let mut env: Vec<String> = std::env::vars()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();

        // Add Ruby-specific env vars to avoid bundler version mismatch issues
        // This is harmless for non-Ruby containers
        env.push("BUNDLE_DISABLE_VERSION_CHECK=true".to_string());

        // Label containers with parent ID for watchdog cleanup (use short form consistently)
        let mut labels = HashMap::new();
        labels.insert("nuanced.role".to_string(), "language-server".to_string());
        // Always tag the parent as this orchestrator instance (short ID)
        labels.insert(
            "nuanced.parent".to_string(),
            self.instance_id_short().clone(),
        );

        // Compute working directory for Sorbet containers with a config directory
        // Sorbet reads sorbet/config which contains "--dir ." (current directory).
        // We need to change the working directory so "." resolves to the right place.
        let working_dir = if let Some(sorbet_dir) = language.sorbet_config_dir() {
            // The sorbet_dir could be either:
            // 1. A container path (starts with /mnt/workspace) - use directly
            // 2. A host path - strip mount_source prefix and prepend /mnt/workspace
            let container_sorbet_path = if sorbet_dir.starts_with("/mnt/workspace") {
                // Already a container path, use as-is
                sorbet_dir.display().to_string()
            } else {
                // Host path - compute relative path and convert to container path
                let relative_path = sorbet_dir
                    .strip_prefix(&mount_source)
                    .unwrap_or(sorbet_dir.as_path());
                format!("/mnt/workspace/{}", relative_path.display())
            };

            log::info!(
                "Sorbet container will run from {} (original: {}, workspace: {})",
                container_sorbet_path,
                sorbet_dir.display(),
                mount_source
            );

            // Only set working_dir if it differs from the default
            if container_sorbet_path != "/mnt/workspace" {
                Some(container_sorbet_path)
            } else {
                None
            }
        } else {
            None
        };

        // Find the right image to use
        let image = find_image(&self.docker, image_name.clone()).await?;

        let config = Config {
            image: Some(image),
            working_dir,
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

        // Release port allocation lock - container has started and port is bound
        drop(_port_lock);

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

        // Store container info immediately after starting
        // The container will become available once it's ready to accept requests
        {
            let mut containers_guard = self.containers.lock().await;
            containers_guard.insert(language.clone(), info.clone());
        }
        log::info!(
            "Container {} for {:?} started at {}, health checks will run in background",
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
    /// Polls indefinitely until the container is healthy. The caller should control
    /// overall timeout by polling the service health endpoint.
    ///
    /// # Arguments
    /// * `info` - Container information including endpoint
    ///
    /// # Returns
    /// * `Ok(())` if container responds with healthy status
    pub async fn check_container_health(
        &self,
        info: &ContainerInfo,
    ) -> Result<(), OrchestratorError> {
        let health_url = format!("{}/health", info.endpoint);
        let client = reqwest::Client::new();

        log::info!("Checking health of {} at {}", info.image_name, health_url);

        // Poll health endpoint until container is ready
        // No timeout - let the caller control overall timeout by polling the service health endpoint
        // Use exponential backoff: start at 1s, double each time, max 15s
        let mut backoff_secs = 1;
        const MAX_BACKOFF_SECS: u64 = 15;

        loop {
            // Check container status before attempting health check
            match self
                .docker
                .inspect_container(&info.container_id, None)
                .await
            {
                Ok(details) => {
                    if let Some(state) = &details.state {
                        if !state.running.unwrap_or(false) {
                            let exit_code = state.exit_code.unwrap_or(-1);
                            let error_msg = state.error.as_deref().unwrap_or("");
                            return Err(OrchestratorError::HealthCheck(format!(
                                "Container {} exited with code {}: {}",
                                info.image_name, exit_code, error_msg
                            )));
                        }
                    }
                }
                Err(e) => {
                    return Err(OrchestratorError::HealthCheck(format!(
                        "Failed to inspect container {}: {}",
                        info.image_name, e
                    )));
                }
            }

            match client
                .get(&health_url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    log::info!("{} container is healthy", info.image_name);
                    return Ok(());
                }
                Ok(response) => {
                    log::debug!(
                        "{} health check returned status: {}",
                        info.image_name,
                        response.status()
                    );
                }
                Err(e) => {
                    log::debug!("Health check attempt failed for {}: {}", info.image_name, e);
                }
            }

            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;

            // Exponential backoff with cap
            backoff_secs = std::cmp::min(backoff_secs * 2, MAX_BACKOFF_SECS);
        }
    }

    /// Get a URL-safe slug for a language
    ///
    /// Used for container naming to create unique, identifiable container names.
    fn language_slug(language: &SupportedLanguages) -> String {
        match language {
            SupportedLanguages::Golang => "golang".to_string(),
            SupportedLanguages::Python => "python".to_string(),
            SupportedLanguages::TypeScriptJavaScript => "typescript".to_string(),
            SupportedLanguages::Rust => "rust".to_string(),
            SupportedLanguages::CPP => "clangd".to_string(),
            SupportedLanguages::Java => "java".to_string(),
            SupportedLanguages::PHP => "php".to_string(),
            SupportedLanguages::CSharp => "csharp".to_string(),
            SupportedLanguages::Ruby {
                version, variant, ..
            } => {
                let ruby_version = version.as_str();
                match variant {
                    LanguageVariant::Standard => format!("ruby-{}", ruby_version),
                    LanguageVariant::Sorbet => format!("ruby-sorbet-{}", ruby_version),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::container::language_image;

    use super::*;

    fn test_golang_image() -> String {
        language_image(&SupportedLanguages::Golang)
    }

    fn test_python_image() -> String {
        language_image(&SupportedLanguages::Python)
    }

    // Unit tests - these don't require Docker

    #[test]
    fn test_image_name_for_language() {
        use super::super::language_image_version;
        let version = language_image_version();

        assert_eq!(
            language_image(&SupportedLanguages::Golang),
            format!("nuanced-lsp-golang:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::Python),
            format!("nuanced-lsp-python:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::TypeScriptJavaScript),
            format!("nuanced-lsp-typescript:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::ruby(
                "3.4.4",
                LanguageVariant::Standard
            )),
            format!("nuanced-lsp-ruby-3.4.4:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::ruby(
                "3.3.6",
                LanguageVariant::Standard
            )),
            format!("nuanced-lsp-ruby-3.3.6:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::ruby("3.4.4", LanguageVariant::Sorbet)),
            format!("nuanced-lsp-ruby-sorbet-3.4.4:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::Rust),
            format!("nuanced-lsp-rust:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::CPP),
            format!("nuanced-lsp-clangd:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::Java),
            format!("nuanced-lsp-java:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::PHP),
            format!("nuanced-lsp-php:{}", version)
        );
        assert_eq!(
            language_image(&SupportedLanguages::CSharp),
            format!("nuanced-lsp-csharp:{}", version)
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
            ContainerOrchestrator::language_slug(&SupportedLanguages::ruby(
                "3.4.4",
                LanguageVariant::Standard
            )),
            "ruby-3.4.4"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::ruby(
                "3.3.6",
                LanguageVariant::Standard
            )),
            "ruby-3.3.6"
        );
        assert_eq!(
            ContainerOrchestrator::language_slug(&SupportedLanguages::ruby(
                "3.4.4",
                LanguageVariant::Sorbet
            )),
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

    #[tokio::test]
    #[cfg_attr(not(feature = "docker-tests"), ignore)]
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
            image_name: test_python_image(),
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
    #[cfg_attr(not(feature = "docker-tests"), ignore)]
    async fn test_remove_container_from_map() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        let info = ContainerInfo {
            container_id: "test-456".to_string(),
            image_name: test_golang_image(),
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
    #[cfg_attr(not(feature = "docker-tests"), ignore)]
    async fn test_all_containers() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        // Start empty
        assert_eq!(orchestrator.all_containers().await.len(), 0);

        // Add two containers
        let info1 = ContainerInfo {
            container_id: "test-1".to_string(),
            image_name: test_python_image(),
            port: 8080,
            endpoint: "http://0.0.0.0:8080".to_string(),
        };
        let info2 = ContainerInfo {
            container_id: "test-2".to_string(),
            image_name: test_golang_image(),
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
    #[cfg_attr(not(feature = "docker-tests"), ignore)]
    async fn test_spawn_container_returns_existing() -> Result<(), OrchestratorError> {
        let orchestrator = ContainerOrchestrator::new().await?;

        // Pre-populate with a "container"
        let existing_info = ContainerInfo {
            container_id: "existing-123".to_string(),
            image_name: test_python_image(),
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
    // 1. Docker images to be built (nuanced-lsp-golang:1.0.0, etc.)
    // 2. Valid workspace path
    // 3. Cleanup of created containers
    //
    // This should be done in a separate integration test suite that:
    // - Builds a minimal test image
    // - Tests the full lifecycle
    // - Ensures cleanup even on failure
}
