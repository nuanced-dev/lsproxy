use bollard::Docker;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api_types::SupportedLanguages;

pub mod http_client;
pub mod orchestrator;

pub use http_client::ContainerHttpClient;

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub container_id: String,
    pub image_name: String,
    pub port: u16,
    pub endpoint: String,
}

pub struct ContainerOrchestrator {
    docker: Arc<Docker>,
    containers: Arc<Mutex<HashMap<SupportedLanguages, ContainerInfo>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("Container health check failed: {0}")]
    HealthCheck(String),

    #[error("Container spawn timeout")]
    SpawnTimeout,

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ContainerOrchestrator {
    /// Create a new ContainerOrchestrator and connect to Docker daemon
    pub async fn new() -> Result<Self, OrchestratorError> {
        // Connect to Docker daemon via Unix socket (macOS/Linux) or named pipe (Windows)
        let docker = Docker::connect_with_local_defaults()?;

        // Verify Docker is accessible by pinging it
        docker.ping().await?;

        Ok(Self {
            docker: Arc::new(docker),
            containers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Initialize workspace by detecting languages and spawning containers upfront
    /// This matches the behavior of the original Manager::start_langservers()
    pub async fn initialize_workspace(&self, workspace_path: &str) -> Result<(), OrchestratorError> {
        use crate::utils::file_utils::search_files;
        use crate::utils::workspace_documents::*;
        use std::path::Path;

        let languages = vec![
            (SupportedLanguages::Python, PYTHON_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::TypeScriptJavaScript, TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Rust, RUST_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::CPP, C_AND_CPP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::CSharp, CSHARP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Java, JAVA_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Golang, GOLANG_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::PHP, PHP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Ruby, RUBY_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::RubySorbet, RUBY_SORBET_FILE_PATTERNS.to_vec()),
        ];

        let mut detected_languages = Vec::new();

        // Detect languages by searching for matching files
        for (language, patterns) in languages {
            let pattern_strings: Vec<String> = patterns.iter().map(|&s| s.to_string()).collect();
            let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS.iter().map(|&s| s.to_string()).collect();

            match search_files(Path::new(workspace_path), pattern_strings, exclude_patterns, true) {
                Ok(files) if !files.is_empty() => {
                    detected_languages.push(language);
                }
                _ => {}
            }
        }

        log::info!("Detected languages in workspace: {:?}", detected_languages);

        // Spawn containers for all detected languages
        for language in detected_languages {
            if self.get_container(&language).await.is_some() {
                continue; // Container already exists
            }

            log::info!("Spawning container for {:?}", language);
            match self.spawn_container(language.clone(), workspace_path).await {
                Ok(info) => {
                    log::info!("Successfully spawned container for {:?} at {}", language, info.endpoint);
                }
                Err(e) => {
                    log::error!("Failed to spawn container for {:?}: {}", language, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Get the Docker client
    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    /// Cleanup all containers
    pub async fn cleanup_all(&self) -> Result<(), OrchestratorError> {
        let containers = self.all_containers().await;
        for (lang, _info) in containers {
            if let Err(e) = self.stop_container(&lang).await {
                log::warn!("Failed to stop container for {:?}: {}", lang, e);
            }
        }
        Ok(())
    }

    /// Stop a specific container
    pub async fn stop_container(&self, language: &SupportedLanguages) -> Result<(), OrchestratorError> {
        use bollard::container::{RemoveContainerOptions, StopContainerOptions};

        if let Some(info) = self.remove_container(language).await {
            // Try graceful stop first
            let stop_options = StopContainerOptions {
                t: 10, // 10 second timeout
            };

            match self.docker.stop_container(&info.container_id, Some(stop_options)).await {
                Ok(_) => log::info!("Stopped container {} for {:?}", info.container_id, language),
                Err(e) => log::warn!("Failed to stop container {}: {}", info.container_id, e),
            }

            // Remove the container
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            self.docker.remove_container(&info.container_id, Some(remove_options)).await?;
            log::info!("Removed container {} for {:?}", info.container_id, language);
        }

        Ok(())
    }

    /// Get a reference to a container by language
    pub async fn get_container(&self, language: &SupportedLanguages) -> Option<ContainerInfo> {
        self.containers.lock().await.get(language).cloned()
    }

    /// Store container information
    pub async fn store_container(&self, language: SupportedLanguages, info: ContainerInfo) {
        self.containers.lock().await.insert(language, info);
    }

    /// Remove container information
    pub async fn remove_container(&self, language: &SupportedLanguages) -> Option<ContainerInfo> {
        self.containers.lock().await.remove(language)
    }

    /// Get all tracked containers
    pub async fn all_containers(&self) -> Vec<(SupportedLanguages, ContainerInfo)> {
        self.containers
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_connection() -> Result<(), OrchestratorError> {
        // This test requires Docker to be running
        let orchestrator = ContainerOrchestrator::new().await?;

        // Verify we can access Docker by checking it was initialized
        assert!(orchestrator.all_containers().await.is_empty());

        Ok(())
    }
}