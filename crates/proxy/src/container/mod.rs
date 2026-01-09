use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use common::api_types::{LanguageVariant, SupportedLanguages};

pub mod http_client;
pub mod language_manager;
pub mod orchestrator;

// Container image configuration
// These correspond to the Docker images built by scripts/build-images.sh --all-services

/// Default version tag for Rust containers (wrapper, proxy, watchdog)
/// Can be overridden with SERVICE_IMAGE_VERSION environment variable at
/// build or runtime.
const DEFAULT_SERVICE_IMAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Override version tag for Rust containers specified at runtime.
const BUILD_SERVICE_IMAGE_VERSION: Option<&'static str> = option_env!("SERVICE_IMAGE_VERSION");

/// Default version tag for language containers (python, ruby, typescript, etc.)
/// Can be overridden with LANGUAGE_IMAGE_VERSION environment variable at build
/// or runtime.
const DEFAULT_LANGUAGE_IMAGE_VERSION: &str = include_str!("../../../../language-image-version");

/// Override version tag for Rust containers specified at runtime.
const BUILD_LANGUAGE_IMAGE_VERSION: Option<&'static str> = option_env!("LANGUAGE_IMAGE_VERSION");

/// Base image names (without version tags)
pub const PROXY_IMAGE_BASE: &str = "nuanced-lsp-proxy";
pub const WRAPPER_IMAGE_BASE: &str = "nuanced-lsp-wrapper";
pub const WATCHDOG_IMAGE_BASE: &str = "nuanced-lsp-watchdog";

/// Default container registry for published images
const DEFAULT_CONTAINER_REGISTRY: &str = "ghcr.io/nuanced-dev";

/// Override container registry specified at build time.
const BUILD_CONTAINER_REGISTRY: Option<&'static str> = option_env!("CONTAINER_REGISTRY");

/// Get language image base name
///
/// Language container images follow the naming convention:
/// - Non-Ruby: nuanced-lsp-{language}
/// - Ruby: nuanced-lsp-ruby-{ruby_version}
/// - Ruby Sorbet: nuanced-lsp-ruby-sorbet-{ruby_version}
pub fn language_image_base(language: &SupportedLanguages) -> String {
    match language {
        SupportedLanguages::Golang => format!("nuanced-lsp-golang"),
        SupportedLanguages::Python => format!("nuanced-lsp-python"),
        SupportedLanguages::TypeScriptJavaScript => {
            format!("nuanced-lsp-typescript")
        }
        SupportedLanguages::Rust => format!("nuanced-lsp-rust"),
        SupportedLanguages::CPP => format!("nuanced-lsp-clangd"),
        SupportedLanguages::Java => format!("nuanced-lsp-java"),
        SupportedLanguages::PHP => format!("nuanced-lsp-php"),
        SupportedLanguages::CSharp => format!("nuanced-lsp-csharp"),
        SupportedLanguages::Ruby {
            version, variant, ..
        } => {
            let ruby_version = version.as_str();
            match variant {
                LanguageVariant::Standard => {
                    format!("nuanced-lsp-ruby-{}", ruby_version)
                }
                LanguageVariant::Sorbet => {
                    format!("nuanced-lsp-ruby-sorbet-{}", ruby_version)
                }
            }
        }
    }
}

/// Get service image version from environment or use default
pub fn service_image_version() -> String {
    // Tags are trimmed in case they end in newlines from files or tool output
    if let Ok(tag) = std::env::var("SERVICE_IMAGE_VERSION") {
        tag.trim().to_string()
    } else if let Some(tag) = BUILD_SERVICE_IMAGE_VERSION {
        tag.trim().to_string()
    } else {
        DEFAULT_SERVICE_IMAGE_VERSION.trim().to_string()
    }
}

/// Get language image version from environment or use default
pub fn language_image_version() -> String {
    // Tags are trimmed in case they end in newlines from files or tool output
    if let Ok(tag) = std::env::var("LANGUAGE_IMAGE_VERSION") {
        tag.trim().to_string()
    } else if let Some(tag) = BUILD_LANGUAGE_IMAGE_VERSION {
        tag.trim().to_string()
    } else {
        DEFAULT_LANGUAGE_IMAGE_VERSION.trim().to_string()
    }
}

/// Get container registry from environment or use default
pub fn container_registry() -> String {
    if let Ok(registry) = std::env::var("CONTAINER_REGISTRY") {
        registry.trim().to_string()
    } else if let Some(registry) = BUILD_CONTAINER_REGISTRY {
        registry.trim().to_string()
    } else {
        DEFAULT_CONTAINER_REGISTRY.to_string()
    }
}

/// Helper functions to get full image names with version tags
pub fn proxy_image() -> String {
    std::env::var("PROXY_IMAGE")
        .unwrap_or_else(|_| format!("{}:{}", PROXY_IMAGE_BASE, service_image_version()))
}

pub fn wrapper_image() -> String {
    std::env::var("WRAPPER_IMAGE")
        .unwrap_or_else(|_| format!("{}:{}", WRAPPER_IMAGE_BASE, service_image_version()))
}

pub fn watchdog_image() -> String {
    std::env::var("WATCHDOG_IMAGE")
        .unwrap_or_else(|_| format!("{}:{}", WATCHDOG_IMAGE_BASE, service_image_version()))
}

pub fn language_image(language: &SupportedLanguages) -> String {
    format!(
        "{}:{}",
        language_image_base(language),
        language_image_version()
    )
}

/// Find the right Docker image for a given image name:tag string
///
/// This method implements a fallback strategy to locate or pull container images:
/// 1. Check if the local image exists - if so, return it
/// 2. Otherwise, check if the image prefixed with the container registry exists locally
/// 3. If not, try to pull the prefixed image from the container registry
/// 4. If pull fails, return an error
///
/// # Arguments
/// * `image` - The image name:tag string (e.g., "nuanced-lsp-wrapper:1.0.0")
///
/// # Returns
/// The image name to use when creating a container
pub async fn find_image(docker: &Docker, image: String) -> Result<String, OrchestratorError> {
    // Check if local image exists
    if docker.inspect_image(&image).await.is_ok() {
        log::debug!("Using local image: {image}");
        return Ok(image);
    }

    // Build the registry-prefixed image name
    let cr = container_registry();
    let registry_image = format!("{cr}/{image}");

    // Check if registry image already exists locally
    if docker.inspect_image(&registry_image).await.is_ok() {
        log::debug!("Using existing registry image: {registry_image}");
        return Ok(registry_image);
    }

    // Try to pull from registry
    log::info!("Try pulling from registry: {registry_image}");

    let create_options = CreateImageOptions {
        from_image: registry_image.clone(),
        ..Default::default()
    };

    let mut stream = docker.create_image(Some(create_options), None, None);
    while let Some(info) = stream.next().await {
        match info {
            Ok(_) => {
                log::info!(
                    "Successfully pulled image from registry: {}",
                    registry_image
                );
                return Ok(registry_image);
            }
            Err(e) => {
                log::error!("Failed to pull image from registry: {registry_image}: {e}");
            }
        }
    }

    return Err(OrchestratorError::ImageNotFound(registry_image));
}

pub use http_client::ContainerHttpClient;

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub container_id: String,
    pub image_name: String,
    pub port: u16,
    pub endpoint: String,
}

#[derive(Clone)]
pub struct ContainerOrchestrator {
    docker: Arc<Docker>,
    containers: Arc<Mutex<HashMap<SupportedLanguages, ContainerInfo>>>,
    container_health: Arc<Mutex<HashMap<SupportedLanguages, ContainerHealthStatus>>>,
    wrapper_container_id: Arc<Mutex<Option<String>>>,
    instance_id: String,
    // Per-language locks to prevent duplicate spawns while allowing concurrent spawns of different languages
    spawning_locks: Arc<Mutex<HashMap<SupportedLanguages, Arc<Mutex<()>>>>>,
    // Global lock for port allocation to prevent port conflicts across all languages
    port_allocation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("Container image not found : {0}")]
    ImageNotFound(String),

    #[error("Container health check failed: {0}")]
    HealthCheck(String),

    #[error("Container spawn timeout")]
    SpawnTimeout,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerHealthStatus {
    Pending,
    Healthy,
    Unhealthy,
}

impl ContainerOrchestrator {
    /// Instance identifier (parent container ID if available, otherwise UUID)
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Mark health status for a container
    async fn set_container_health(
        &self,
        language: SupportedLanguages,
        status: ContainerHealthStatus,
    ) {
        let mut guard = self.container_health.lock().await;
        guard.insert(language, status);
    }

    /// Get all languages with tracked health status
    pub async fn get_all_containers_health(
        &self,
    ) -> HashMap<SupportedLanguages, ContainerHealthStatus> {
        self.container_health.lock().await.clone()
    }

    /// Get the short-form identifier used for labels/names (12 chars)
    fn instance_id_short(&self) -> String {
        self.instance_id.chars().take(12).collect()
    }

    /// Create a new ContainerOrchestrator and connect to Docker daemon
    pub async fn new() -> Result<Self, OrchestratorError> {
        // Connect to Docker daemon via Unix socket (macOS/Linux) or named pipe (Windows)
        let docker = Docker::connect_with_local_defaults()?;

        // Verify Docker is accessible by pinging it
        docker.ping().await?;

        // Use parent container ID if available; otherwise fallback to random UUID
        let instance_id =
            Self::get_own_container_id().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(Self {
            docker: Arc::new(docker),
            containers: Arc::new(Mutex::new(HashMap::new())),
            container_health: Arc::new(Mutex::new(HashMap::new())),
            wrapper_container_id: Arc::new(Mutex::new(None)),
            instance_id,
            spawning_locks: Arc::new(Mutex::new(HashMap::new())),
            port_allocation_lock: Arc::new(Mutex::new(())),
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
    ///
    /// For languages with version support (like Ruby), returns the default version.
    /// This is used for parsing the ENABLED_LANGUAGES environment variable.
    fn parse_language(s: &str) -> Option<SupportedLanguages> {
        match s.trim().to_lowercase().as_str() {
            "python" => Some(SupportedLanguages::Python),
            "typescript" | "javascript" => Some(SupportedLanguages::TypeScriptJavaScript),
            "rust" => Some(SupportedLanguages::Rust),
            "cpp" | "c++" | "c" => Some(SupportedLanguages::CPP),
            "csharp" | "c#" => Some(SupportedLanguages::CSharp),
            "java" => Some(SupportedLanguages::Java),
            "go" => Some(SupportedLanguages::Golang),
            "php" => Some(SupportedLanguages::PHP),
            "ruby" => Some(SupportedLanguages::ruby_default()),
            "ruby-sorbet" | "sorbet" => Some(SupportedLanguages::ruby_sorbet_default()),
            _ => None,
        }
    }

    /// Get the set of enabled languages from the ENABLED_LANGUAGES environment variable
    /// Returns None if the variable is not set (all languages enabled)
    /// Returns Some(HashSet) with the parsed languages if set
    fn get_enabled_languages() -> Option<HashSet<SupportedLanguages>> {
        std::env::var("ENABLED_LANGUAGES")
            .ok()
            .map(|enabled_langs| {
                enabled_langs
                    .split(',')
                    .filter_map(Self::parse_language)
                    .collect()
            })
    }

    /// Initialize workspace by detecting languages and spawning containers upfront
    /// This matches the behavior of the original Manager::start_langservers()
    pub async fn initialize_workspace(
        &self,
        workspace_path: &str,
    ) -> Result<(), OrchestratorError> {
        use crate::container::language_manager::*;
        use common::utils::file_utils::search_files;
        use common::utils::workspace_documents::DEFAULT_EXCLUDE_PATTERNS;
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

        log::info!(
            "Scanning workspace with {} language managers",
            managers.len()
        );

        // Collect all patterns from all managers
        let all_patterns: Vec<String> = managers.iter().flat_map(|m| m.file_patterns()).collect();

        log::info!("Scanning for {} file patterns", all_patterns.len());
        log::debug!("File patterns: {:?}", all_patterns);

        // Single workspace scan
        let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        let files = search_files(
            Path::new(workspace_path),
            all_patterns,
            exclude_patterns,
            false, // Don't respect gitignore - we need to detect all manifest files including dotfiles
        )?;

        log::info!("Found {} files in workspace", files.len());
        log::debug!("Files found: {:?}", files);

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
        let languages_to_spawn: Vec<SupportedLanguages> = if let Some(enabled) = &enabled_languages
        {
            log::info!("Filtering detected languages. Enabled: {:?}", enabled);
            detected_languages
                .into_iter()
                .filter(|lang| {
                    // Check if this language matches any enabled language family
                    // e.g., Ruby3_2_6 matches if "ruby" (Ruby3_4_4) is enabled
                    enabled.iter().any(|family| lang.matches_family(family))
                })
                .collect()
        } else {
            detected_languages
        };

        log::info!("Languages to spawn: {:?}", languages_to_spawn);

        // Filter out languages that already have containers
        let mut languages_needing_spawn = Vec::new();
        for language in languages_to_spawn {
            if self.get_container(&language).await.is_none() {
                languages_needing_spawn.push(language);
            }
        }

        if languages_needing_spawn.is_empty() {
            log::info!("All language containers already exist");
            return Ok(());
        }

        log::info!(
            "Spawning {} containers in parallel",
            languages_needing_spawn.len()
        );

        // Spawn all containers in parallel
        let spawn_futures: Vec<_> = languages_needing_spawn
            .iter()
            .cloned()
            .map(|language| {
                let orchestrator = self.clone();
                async move {
                    log::info!("Spawning container for {:?}", language);
                    let result = orchestrator.spawn_container(language.clone()).await;
                    (language, result)
                }
            })
            .collect();

        // Wait for all spawns to complete
        let results = futures::future::join_all(spawn_futures).await;

        // Check results - log successes and failures
        for (language, result) in results {
            match result {
                Ok(info) => {
                    log::info!(
                        "Container for {:?} created at {}, health checks running",
                        language,
                        info.endpoint
                    );
                }
                Err(e) => {
                    log::error!("Failed to spawn container for {:?}: {}", language, e);
                }
            }
        }

        // Wait for all initialization containers to report non-Pending status
        log::info!("Waiting for all language containers to report their status...");
        loop {
            let all_ready;
            {
                let health_statuses = self.container_health.lock().await;
                all_ready = health_statuses
                    .values()
                    .all(|status| *status != ContainerHealthStatus::Pending);
                // lock is released
            }

            if all_ready {
                log::info!("All language containers have reported their status");
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Get the Docker client
    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    /// Spawn the wrapper container
    /// The wrapper container holds the lsp-wrapper binary and ast-grep configs
    /// that will be mounted into language containers via --volumes-from
    pub async fn ensure_wrapper_container(&self) -> Result<String, OrchestratorError> {
        use bollard::container::{Config, CreateContainerOptions};
        use bollard::models::HostConfig;

        // Hold lock for entire function to prevent concurrent wrapper creation
        let mut wrapper_id = self.wrapper_container_id.lock().await;

        // Check if we already have a wrapper container ID
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

        let id_short = self.instance_id_short();
        let wrapper_name = format!("nuanced-lsp-wrapper-{}", id_short);

        // Check if wrapper container already exists (by name)
        if let Ok(info) = self.docker.inspect_container(&wrapper_name, None).await {
            if let Some(state) = info.state {
                if state.running == Some(true) {
                    if let Some(id) = info.id {
                        log::info!("Found existing wrapper container: {}", id);
                        *wrapper_id = Some(id.clone());
                        return Ok(id);
                    }
                }
            }
            // Container exists but not running - try to start it
            if let Some(id) = info.id {
                log::info!("Starting existing wrapper container: {}", id);
                self.docker.start_container::<String>(&id, None).await?;
                *wrapper_id = Some(id.clone());
                return Ok(id);
            }
        }

        // Create new wrapper container
        log::info!("Creating new wrapper container");

        // Find the right image to use
        let image = find_image(&self.docker, wrapper_image()).await?;

        let config = Config {
            image: Some(image),
            labels: Some({
                let mut l = HashMap::new();
                l.insert("nuanced.role".to_string(), "wrapper".to_string());
                l.insert("nuanced.parent".to_string(), id_short.clone());
                l
            }),
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

        // Store container ID (lock already held)
        *wrapper_id = Some(container_id.clone());

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

            match self
                .docker
                .remove_container(&id, Some(remove_options))
                .await
            {
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
    pub async fn stop_container(
        &self,
        language: &SupportedLanguages,
    ) -> Result<(), OrchestratorError> {
        use bollard::container::{RemoveContainerOptions, StopContainerOptions};

        if let Some(info) = self.remove_container(language).await {
            // Try graceful stop first
            let stop_options = StopContainerOptions {
                t: 10, // 10 second timeout
            };

            match self
                .docker
                .stop_container(&info.container_id, Some(stop_options))
                .await
            {
                Ok(_) => log::info!("Stopped container {} for {:?}", info.container_id, language),
                Err(e) => log::warn!("Failed to stop container {}: {}", info.container_id, e),
            }

            // Remove the container
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            self.docker
                .remove_container(&info.container_id, Some(remove_options))
                .await?;
            log::info!("Removed container {} for {:?}", info.container_id, language);
        }

        Ok(())
    }

    /// Spawn a watchdog container to monitor this service and cleanup on unexpected death
    /// Returns the watchdog container ID
    pub async fn spawn_watchdog(&self) -> Result<String, OrchestratorError> {
        use bollard::container::{Config, CreateContainerOptions};
        use bollard::models::HostConfig;

        let parent_id = self.instance_id_short();

        log::info!(
            "Spawning watchdog to monitor parent container: {}",
            parent_id
        );

        let watchdog_name = format!("nuanced-lsp-watchdog-{}", self.instance_id_short());

        // Check if watchdog already exists
        if let Ok(_) = self.docker.inspect_container(&watchdog_name, None).await {
            log::info!("Watchdog already exists: {}", watchdog_name);
            return Ok(watchdog_name);
        }

        // Find the right image to use
        let image = find_image(&self.docker, watchdog_image()).await?;

        let config = Config {
            image: Some(image),
            env: Some(vec![format!("PARENT_CONTAINER_ID={}", parent_id)]),
            host_config: Some(HostConfig {
                binds: Some(vec!["/var/run/docker.sock:/var/run/docker.sock".to_string()]),
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

        self.docker
            .start_container::<String>(&container.id, None)
            .await?;

        log::info!(
            "Watchdog spawned: {} (monitoring {})",
            watchdog_name,
            parent_id
        );

        Ok(watchdog_name)
    }

    /// Stop the watchdog container
    pub async fn stop_watchdog(&self) -> Result<(), OrchestratorError> {
        use bollard::container::RemoveContainerOptions;

        // Use instance_id (parent container ID preferred) for unique watchdog name
        let watchdog_name = format!("nuanced-lsp-watchdog-{}", self.instance_id_short());

        // Try to remove the watchdog (force=true handles running containers)
        let remove_options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        match self
            .docker
            .remove_container(&watchdog_name, Some(remove_options))
            .await
        {
            Ok(_) => log::info!("Stopped watchdog: {}", watchdog_name),
            Err(e) => log::debug!("Watchdog removal failed (may not exist): {}", e),
        }

        Ok(())
    }

    /// Get a reference to a container by language
    pub async fn get_container(&self, language: &SupportedLanguages) -> Option<ContainerInfo> {
        self.containers.lock().await.get(language).cloned()
    }

    /// Store container information
    pub async fn store_container(&self, language: SupportedLanguages, info: ContainerInfo) {
        self.containers.lock().await.insert(language.clone(), info);
        self.container_health
            .lock()
            .await
            .insert(language, ContainerHealthStatus::Pending);
    }

    /// Remove container information
    pub async fn remove_container(&self, language: &SupportedLanguages) -> Option<ContainerInfo> {
        let removed = self.containers.lock().await.remove(language);
        self.container_health.lock().await.remove(language);
        removed
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
    #[cfg_attr(not(feature = "docker-tests"), ignore)]
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

        // Test go language (no golang alias - we standardized on "go")
        assert_eq!(
            ContainerOrchestrator::parse_language("go"),
            Some(SupportedLanguages::Golang)
        );
        // Verify "golang" is NOT accepted (removed for consistency)
        assert_eq!(ContainerOrchestrator::parse_language("golang"), None);
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
            Some(SupportedLanguages::ruby_sorbet_default())
        );
        assert_eq!(
            ContainerOrchestrator::parse_language("sorbet"),
            Some(SupportedLanguages::ruby_sorbet_default())
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
