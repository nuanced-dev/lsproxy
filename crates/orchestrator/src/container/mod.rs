use bollard::Docker;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use lsproxy_common::api_types::SupportedLanguages;

pub mod http_client;
pub mod language_manager;
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
    wrapper_container_id: Arc<Mutex<Option<String>>>,
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
            wrapper_container_id: Arc::new(Mutex::new(None)),
        })
    }

    /// Get this service container's own ID
    /// Returns None if not running in a container
    pub fn get_own_container_id() -> Option<String> {
        // Docker sets HOSTNAME to the container ID (short form)
        // We can also get the full ID from /proc/self/cgroup
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            // Validate it looks like a container ID (12 hex chars)
            if hostname.len() >= 12 && hostname.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(hostname);
            }
        }

        // Fallback: Parse container ID from cgroup (Linux only)
        #[cfg(target_os = "linux")]
        {
            if let Ok(cgroup) = std::fs::read_to_string("/proc/self/cgroup") {
                // Look for docker container ID in cgroup path
                // Format: .../docker/<container_id>/...
                for line in cgroup.lines() {
                    if let Some(docker_part) = line.split("docker/").nth(1) {
                        if let Some(id) = docker_part.split('/').next() {
                            if id.len() >= 12 {
                                return Some(id.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Get the host path for the workspace mount by inspecting our own container
    ///
    /// When the service spawns language containers via Docker API, it needs to mount the
    /// workspace directory into those containers. Docker interprets mount paths from the
    /// HOST's perspective, not from inside the service container.
    ///
    /// For example:
    /// - Host has workspace at: `/Users/user/project`
    /// - Service container sees it at: `/mnt/workspace` (via bind mount)
    /// - When spawning a language container, we must tell Docker to mount `/Users/user/project`
    ///   (the host path), not `/mnt/workspace` (which doesn't exist on the host)
    ///
    /// This method inspects the service container's own mounts to discover the original
    /// host path, eliminating the need for users to manually pass HOST_WORKSPACE_PATH.
    ///
    /// The caller should error if this returns None and HOST_WORKSPACE_PATH is not set,
    /// as the host workspace path is required for spawning language containers.
    ///
    /// Returns None if not running in a container or mount not found
    pub async fn get_host_workspace_path(&self) -> Option<String> {
        let container_id = Self::get_own_container_id()?;

        match self.docker.inspect_container(&container_id, None).await {
            Ok(inspect) => {
                if let Some(mounts) = inspect.mounts {
                    // Look for the mount with destination /mnt/workspace
                    for mount in mounts {
                        if mount.destination.as_deref() == Some("/mnt/workspace") {
                            if let Some(source) = mount.source {
                                log::info!("Auto-detected host workspace path: {}", source);
                                return Some(source);
                            }
                        }
                    }
                }
                log::warn!("Could not find /mnt/workspace mount in own container");
                None
            }
            Err(e) => {
                log::warn!("Failed to inspect own container {}: {}", container_id, e);
                None
            }
        }
    }

    /// Parse a language string (case-insensitive, handles aliases)
    fn parse_language(s: &str) -> Option<SupportedLanguages> {
        match s.trim().to_lowercase().as_str() {
            "python" => Some(SupportedLanguages::Python),
            "typescript" | "javascript" => Some(SupportedLanguages::TypeScriptJavaScript),
            "rust" => Some(SupportedLanguages::Rust),
            "cpp" | "c++" | "c" => Some(SupportedLanguages::CPP),
            "csharp" | "c#" => Some(SupportedLanguages::CSharp),
            "java" => Some(SupportedLanguages::Java),
            "golang" | "go" => Some(SupportedLanguages::Golang),
            "php" => Some(SupportedLanguages::PHP),
            "ruby" => Some(SupportedLanguages::Ruby3_4_4),
            "ruby-sorbet" | "sorbet" => Some(SupportedLanguages::RubySorbet3_4_4),
            _ => None,
        }
    }

    /// Get the set of enabled languages from the ENABLED_LANGUAGES environment variable
    /// Returns None if the variable is not set (all languages enabled)
    /// Returns Some(HashSet) with the parsed languages if set
    fn get_enabled_languages() -> Option<HashSet<SupportedLanguages>> {
        std::env::var("ENABLED_LANGUAGES").ok().map(|enabled_langs| {
            enabled_langs
                .split(',')
                .filter_map(Self::parse_language)
                .collect()
        })
    }

    /// Initialize workspace by detecting languages and spawning containers upfront
    /// This matches the behavior of the original Manager::start_langservers()
    pub async fn initialize_workspace(&self, workspace_path: &str) -> Result<(), OrchestratorError> {
        use crate::container::language_manager::*;
        use lsproxy_common::utils::file_utils::search_files;
        use lsproxy_common::utils::workspace_documents::DEFAULT_EXCLUDE_PATTERNS;
        use std::path::Path;

        // Create all language managers
        let mut managers: Vec<Box<dyn LanguageManager>> = vec![
            Box::new(RubyManager::new()),
            Box::new(PythonManager::new()),
            Box::new(TypeScriptJavaScriptManager::new()),
            Box::new(RustManager::new()),
            Box::new(CPPManager::new()),
            Box::new(CSharpManager::new()),
            Box::new(JavaManager::new()),
            Box::new(GolangManager::new()),
            Box::new(PHPManager::new()),
        ];

        log::info!("Scanning workspace with {} language managers", managers.len());

        // Collect all patterns from all managers
        let all_patterns: Vec<String> = managers
            .iter()
            .flat_map(|m| m.file_patterns())
            .collect();

        log::info!("Scanning for {} file patterns", all_patterns.len());

        // Single workspace scan
        let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        let files = search_files(
            Path::new(workspace_path),
            all_patterns,
            exclude_patterns,
            true,
        )?;

        log::info!("Found {} files in workspace", files.len());

        // Each manager processes each file
        for file_path in &files {
            for manager in &mut managers {
                manager.process_file(file_path);
            }
        }

        // Collect detected languages from all managers
        let detected_languages: Vec<SupportedLanguages> = managers
            .iter()
            .flat_map(|m| {
                let langs = m.finalize();
                if !langs.is_empty() {
                    log::info!("{} detected: {:?}", m.name(), langs);
                }
                langs
            })
            .collect();

        log::info!("Total detected languages: {:?}", detected_languages);

        // Filter based on ENABLED_LANGUAGES environment variable
        let enabled_languages = Self::get_enabled_languages();
        let languages_to_spawn: Vec<SupportedLanguages> = if let Some(enabled) = &enabled_languages {
            log::info!("Filtering detected languages. Enabled: {:?}", enabled);
            detected_languages
                .into_iter()
                .filter(|lang| enabled.contains(lang))
                .collect()
        } else {
            detected_languages
        };

        log::info!("Languages to spawn: {:?}", languages_to_spawn);

        // Spawn containers for filtered languages
        for language in languages_to_spawn {
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

    /// Spawn or get existing wrapper container
    /// The wrapper container holds the lsp-wrapper binary and ast-grep configs
    /// that will be mounted into language containers via --volumes-from
    pub async fn ensure_wrapper_container(&self) -> Result<String, OrchestratorError> {
        // Check if we already have a wrapper container ID
        {
            let wrapper_id = self.wrapper_container_id.lock().await;
            if let Some(id) = wrapper_id.as_ref() {
                // Verify container still exists and is running
                if let Ok(info) = self.docker.inspect_container(id, None).await {
                    if let Some(state) = info.state {
                        if state.running == Some(true) {
                            log::debug!("Using existing wrapper container: {}", id);
                            return Ok(id.clone());
                        }
                    }
                }
                // Container no longer valid, will create new one
                log::warn!("Wrapper container {} is not running, creating new one", id);
            }
        }

        use bollard::container::{Config, CreateContainerOptions};
        use bollard::models::HostConfig;

        let wrapper_name = "lsproxy-wrapper";

        // Check if wrapper container already exists (by name)
        if let Ok(info) = self.docker.inspect_container(wrapper_name, None).await {
            if let Some(state) = info.state {
                if state.running == Some(true) {
                    if let Some(id) = info.id {
                        log::info!("Found existing wrapper container: {}", id);
                        *self.wrapper_container_id.lock().await = Some(id.clone());
                        return Ok(id);
                    }
                }
            }
            // Container exists but not running - try to start it
            if let Some(id) = info.id {
                log::info!("Starting existing wrapper container: {}", id);
                self.docker.start_container::<String>(&id, None).await?;
                *self.wrapper_container_id.lock().await = Some(id.clone());
                return Ok(id);
            }
        }

        // Create new wrapper container
        log::info!("Creating new wrapper container");

        let config = Config {
            image: Some("lsproxy-wrapper:latest".to_string()),
            host_config: Some(HostConfig {
                auto_remove: Some(false), // Keep container around for volume sharing
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: wrapper_name.to_string(),
            ..Default::default()
        };

        let container = self.docker.create_container(Some(options), config).await?;
        let container_id = container.id;

        // Start the wrapper container (runs sleep infinity to stay alive)
        log::info!("Starting wrapper container: {}", container_id);
        self.docker
            .start_container::<String>(&container_id, None)
            .await?;

        // Store container ID
        *self.wrapper_container_id.lock().await = Some(container_id.clone());

        log::info!("Wrapper container started: {}", container_id);
        Ok(container_id)
    }

    /// Stop and remove the wrapper container
    pub async fn stop_wrapper_container(&self) -> Result<(), OrchestratorError> {
        use bollard::container::RemoveContainerOptions;

        let wrapper_id = {
            let mut wrapper = self.wrapper_container_id.lock().await;
            wrapper.take()
        };

        if let Some(id) = wrapper_id {
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            match self.docker.remove_container(&id, Some(remove_options)).await {
                Ok(_) => log::info!("Stopped wrapper container: {}", id),
                Err(e) => log::warn!("Failed to remove wrapper container {}: {}", id, e),
            }
        }

        Ok(())
    }

    /// Cleanup all containers including wrapper
    pub async fn cleanup_all(&self) -> Result<(), OrchestratorError> {
        let containers = self.all_containers().await;
        for (lang, _info) in containers {
            if let Err(e) = self.stop_container(&lang).await {
                log::warn!("Failed to stop container for {:?}: {}", lang, e);
            }
        }

        // Stop wrapper container
        if let Err(e) = self.stop_wrapper_container().await {
            log::warn!("Failed to stop wrapper container: {}", e);
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

    /// Spawn a watchdog container to monitor this service and cleanup on unexpected death
    /// Returns the watchdog container ID
    pub async fn spawn_watchdog(&self) -> Result<String, OrchestratorError> {
        use bollard::container::{Config, CreateContainerOptions};
        use bollard::models::HostConfig;

        let parent_id = Self::get_own_container_id()
            .ok_or_else(|| OrchestratorError::Network("Cannot determine own container ID for watchdog".to_string()))?;

        log::info!("Spawning watchdog to monitor parent container: {}", parent_id);

        let watchdog_name = format!("lsproxy-watchdog-{}", &parent_id[..12]);

        // Check if watchdog already exists
        if let Ok(_) = self.docker.inspect_container(&watchdog_name, None).await {
            log::info!("Watchdog already exists: {}", watchdog_name);
            return Ok(watchdog_name);
        }

        let config = Config {
            image: Some("lsproxy-watchdog:latest".to_string()),
            env: Some(vec![
                format!("PARENT_CONTAINER_ID={}", parent_id)
            ]),
            host_config: Some(HostConfig {
                binds: Some(vec![
                    "/var/run/docker.sock:/var/run/docker.sock".to_string()
                ]),
                auto_remove: Some(true), // Auto-remove container when it exits
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: watchdog_name.clone(),
            ..Default::default()
        };

        let container = self.docker.create_container(Some(options), config).await?;
        self.docker.start_container::<String>(&container.id, None).await?;

        log::info!("Watchdog spawned: {} (monitoring {})", watchdog_name, parent_id);

        Ok(watchdog_name)
    }

    /// Stop the watchdog container
    pub async fn stop_watchdog(&self) -> Result<(), OrchestratorError> {
        use bollard::container::RemoveContainerOptions;

        if let Some(parent_id) = Self::get_own_container_id() {
            let watchdog_name = format!("lsproxy-watchdog-{}", &parent_id[..12]);

            // Try to remove the watchdog (force=true handles running containers)
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            match self.docker.remove_container(&watchdog_name, Some(remove_options)).await {
                Ok(_) => log::info!("Stopped watchdog: {}", watchdog_name),
                Err(e) => log::debug!("Watchdog removal failed (may not exist): {}", e),
            }
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
    use serial_test::serial;

    #[tokio::test]
    async fn test_docker_connection() -> Result<(), OrchestratorError> {
        // This test requires Docker to be running
        let orchestrator = ContainerOrchestrator::new().await?;

        // Verify we can access Docker by checking it was initialized
        assert!(orchestrator.all_containers().await.is_empty());

        Ok(())
    }

    #[test]
    fn test_parse_language() {
        // Test basic language names
        assert_eq!(
            ContainerOrchestrator::parse_language("python"),
            Some(SupportedLanguages::Python)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("rust"),
            Some(SupportedLanguages::Rust)
        );

        // Test case insensitivity
        assert_eq!(
            ContainerOrchestrator::parse_language("PYTHON"),
            Some(SupportedLanguages::Python)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("RuSt"),
            Some(SupportedLanguages::Rust)
        );

        // Test whitespace handling
        assert_eq!(
            ContainerOrchestrator::parse_language(" python "),
            Some(SupportedLanguages::Python)
        );

        // Test aliases
        assert_eq!(
            ContainerOrchestrator::parse_language("golang"),
            Some(SupportedLanguages::Golang)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("go"),
            Some(SupportedLanguages::Golang)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("cpp"),
            Some(SupportedLanguages::CPP)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("c++"),
            Some(SupportedLanguages::CPP)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("c"),
            Some(SupportedLanguages::CPP)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("csharp"),
            Some(SupportedLanguages::CSharp)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("c#"),
            Some(SupportedLanguages::CSharp)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("javascript"),
            Some(SupportedLanguages::TypeScriptJavaScript)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("typescript"),
            Some(SupportedLanguages::TypeScriptJavaScript)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("ruby-sorbet"),
            Some(SupportedLanguages::RubySorbet3_4_4)
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("sorbet"),
            Some(SupportedLanguages::RubySorbet3_4_4)
        );

        // Test invalid language
        assert_eq!(ContainerOrchestrator::parse_language("invalid"), None);
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_not_set() {
        // Ensure variable is not set
        std::env::remove_var("ENABLED_LANGUAGES");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_none(), "Should return None when variable not set");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_single() {
        std::env::set_var("ENABLED_LANGUAGES", "python");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_some());

        let languages = result.unwrap();
        assert_eq!(languages.len(), 1);
        assert!(languages.contains(&SupportedLanguages::Python));

        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_multiple() {
        std::env::set_var("ENABLED_LANGUAGES", "python,rust,typescript");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_some());

        let languages = result.unwrap();
        assert_eq!(languages.len(), 3);
        assert!(languages.contains(&SupportedLanguages::Python));
        assert!(languages.contains(&SupportedLanguages::Rust));
        assert!(languages.contains(&SupportedLanguages::TypeScriptJavaScript));

        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_with_spaces() {
        std::env::set_var("ENABLED_LANGUAGES", " python , rust , go ");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_some());

        let languages = result.unwrap();
        assert_eq!(languages.len(), 3);
        assert!(languages.contains(&SupportedLanguages::Python));
        assert!(languages.contains(&SupportedLanguages::Rust));
        assert!(languages.contains(&SupportedLanguages::Golang));

        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_with_invalid() {
        std::env::set_var("ENABLED_LANGUAGES", "python,invalid,rust");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_some());

        let languages = result.unwrap();
        // Invalid languages are filtered out
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&SupportedLanguages::Python));
        assert!(languages.contains(&SupportedLanguages::Rust));

        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_empty_string() {
        std::env::set_var("ENABLED_LANGUAGES", "");

        let result = ContainerOrchestrator::get_enabled_languages();
        assert!(result.is_some());

        let languages = result.unwrap();
        // Empty string results in empty set
        assert_eq!(languages.len(), 0);

        std::env::remove_var("ENABLED_LANGUAGES");
    }
}