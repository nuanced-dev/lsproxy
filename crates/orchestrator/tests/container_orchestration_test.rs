/// Integration tests for containerized LSProxy architecture
///
/// Tests the full flow:
/// 1. Base LSProxy service running in container
/// 2. Dynamic spawning of language-specific containers (Python)
/// 3. Request forwarding and response handling
/// 4. Container lifecycle management
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::json;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::sleep;

use lsproxy_orchestrator::container::{
    proxy_image, wrapper_image, LANGUAGE_CONTAINER_VERSION, WRAPPER_IMAGE_BASE,
};

// Helper function for Python test image
fn python_image() -> String {
    format!("nuanced-lsp-python:{}", LANGUAGE_CONTAINER_VERSION)
}
const SERVICE_PORT: u16 = 14444; // Use non-standard port to avoid conflicts
const CONTAINER_PORT: u16 = 4444; // Port the service listens on inside container
const BASE_URL: &str = "http://localhost:14444";
const MAX_RETRIES: u32 = 30;
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Shared test fixture that lives for the entire test suite
/// Uses Lazy initialization to set up once and reuse across all serial tests
static SUITE_FIXTURE: Lazy<Arc<Mutex<Option<ContainerFixture>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Test fixture that manages container lifecycle
struct ContainerFixture {
    docker: Docker,
    // Keep workspace_dir alive for the duration of tests
    // TempDir's Drop impl deletes the directory when it goes out of scope
    #[allow(dead_code)]
    workspace_dir: TempDir,
}

impl ContainerFixture {
    /// Comprehensive cleanup of all test-related containers
    /// Removes orphaned containers from previous failed test runs
    async fn cleanup_all_test_containers(
        docker: &Docker,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Cleaning up all test-related containers...");

        // Clean up all nuanced-lsp-python-* containers (test language containers)
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec!["nuanced-lsp-python-".to_string()]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let python_containers = docker.list_containers(Some(options)).await?;
        for container in python_containers {
            if let Some(id) = container.id {
                let _ = docker
                    .remove_container(
                        &id,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            }
        }

        // Clean up test watchdog containers
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec!["nuanced-lsp-watchdog-".to_string()]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let watchdog_containers = docker.list_containers(Some(options)).await?;
        for container in watchdog_containers {
            if let Some(id) = container.id {
                let _ = docker
                    .remove_container(
                        &id,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            }
        }

        // Clean up wrapper container
        let _ = docker
            .remove_container(
                WRAPPER_IMAGE_BASE,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        // Clean up test service container
        let _ = docker
            .remove_container(
                "nuanced-lsp-test-service",
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        println!("Cleanup complete");
        Ok(())
    }

    /// Create new test fixture with workspace and start the service
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let docker = Docker::connect_with_socket_defaults()?;

        // Clean up all test-related containers from previous runs
        Self::cleanup_all_test_containers(&docker).await?;

        // Verify required images exist
        Self::verify_images(&docker).await?;

        let workspace_dir = tempfile::tempdir()?;

        // Create test Python files
        Self::create_test_files(&workspace_dir)?;

        // Start the service
        Self::start_service_internal(&docker, &workspace_dir).await?;

        Ok(Self {
            docker,
            workspace_dir,
        })
    }

    /// Verify required Docker images are available
    async fn verify_images(docker: &Docker) -> Result<(), Box<dyn std::error::Error>> {
        let proxy_img = proxy_image();
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![proxy_img.clone()]);

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = docker.list_images(Some(options)).await?;
        if images.is_empty() {
            return Err(format!("Required image {} not found. Run: ./scripts/build-rust-containers.sh", proxy_img).into());
        }

        let python_img = python_image();
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![python_img.clone()]);

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = docker.list_images(Some(options)).await?;
        if images.is_empty() {
            return Err(format!("Required image {} not found. Run: ./scripts/build-language-containers.sh", python_img).into());
        }

        Ok(())
    }

    /// Create test Python files in workspace
    fn create_test_files(workspace: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
        let test_file = workspace.path().join("test.py");
        std::fs::write(&test_file,
            "def hello():\n    return \"hello\"\n\ndef world():\n    return \"world\"\n\nmessage = hello()\n"
        )?;

        let simple_file = workspace.path().join("simple.py");
        std::fs::write(&simple_file, "def foo(): pass\n")?;

        Ok(())
    }

    /// Start the base LSProxy service container (internal helper)
    async fn start_service_internal(
        docker: &Docker,
        workspace_dir: &TempDir,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_path = workspace_dir
            .path()
            .to_str()
            .ok_or("Invalid workspace path")?;

        let config = Config {
            image: Some(&proxy_image()),
            env: Some(vec!["USE_AUTH=false", "RUST_LOG=info"]),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![
                    "/var/run/docker.sock:/var/run/docker.sock".to_string(),
                    format!("{}:/mnt/workspace", workspace_path),
                ]),
                port_bindings: Some(
                    [(
                        format!("{}/tcp", CONTAINER_PORT),
                        Some(vec![bollard::models::PortBinding {
                            host_ip: Some("0.0.0.0".to_string()),
                            host_port: Some(SERVICE_PORT.to_string()),
                        }]),
                    )]
                    .into_iter()
                    .collect(),
                ),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
                ..Default::default()
            }),
            exposed_ports: None,
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: "nuanced-lsp-test-service",
            ..Default::default()
        };

        let container = docker.create_container(Some(options), config).await?;

        docker
            .start_container::<String>(&container.id, None)
            .await?;

        // Wait for service to be healthy
        Self::wait_for_health_static().await?;

        Ok(())
    }

    /// Wait for service health check to pass (static version)
    async fn wait_for_health_static() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

        let health_url = format!("{}/v1/system/health", BASE_URL);

        for attempt in 1..=MAX_RETRIES {
            match client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(health) = response.json::<serde_json::Value>().await {
                        if health["status"] == "ok" {
                            println!("Service healthy after {} attempts", attempt);
                            return Ok(());
                        }
                    }
                }
                Ok(response) => {
                    println!("Health check returned status: {}", response.status());
                }
                Err(e) => {
                    println!("Health check attempt {}/{}: {}", attempt, MAX_RETRIES, e);
                }
            }

            if attempt < MAX_RETRIES {
                sleep(RETRY_DELAY).await;
            }
        }

        Err("Service did not become healthy within timeout".into())
    }

    /// Clean up all test containers (for final teardown)
    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        Self::cleanup_all_test_containers(&self.docker).await
    }
}

/// Get or initialize the shared test fixture
/// This runs once for the entire test suite
async fn get_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture_guard = SUITE_FIXTURE.lock().await;

    if fixture_guard.is_none() {
        println!("Initializing shared test fixture...");
        let fixture = ContainerFixture::new().await?;
        *fixture_guard = Some(fixture);
        println!("Test fixture ready");
    }

    Ok(())
}

/// Cleanup the shared test fixture (called at end of test suite)
async fn cleanup_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture_guard = SUITE_FIXTURE.lock().await;

    if let Some(fixture) = fixture_guard.take() {
        println!("Cleaning up shared test fixture...");
        fixture.cleanup().await?;
        println!("Cleanup complete");
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_service_health() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let client = Client::new();
    let response = client
        .get(&format!("{}/v1/system/health", BASE_URL))
        .send()
        .await?;

    assert!(response.status().is_success());

    let health: serde_json::Value = response.json().await?;
    assert_eq!(health["status"], "ok");
    // In the containerized architecture, languages are spawned dynamically, not built-in
    assert!(health["languages"].is_object());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_container_spawn_on_request() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let docker = Docker::connect_with_socket_defaults()?;

    // With eager initialization, Python container should be spawned during service startup
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec!["nuanced-lsp-python-".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);
    let options = ListContainersOptions {
        filters,
        ..Default::default()
    };
    let initial_containers: Vec<String> = docker
        .list_containers(Some(options.clone()))
        .await?
        .iter()
        .filter_map(|c| c.id.clone())
        .collect();
    assert_eq!(
        initial_containers.len(),
        1,
        "Expected exactly one Python container after service startup"
    );

    // Make a request - should use the existing container
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let response = client
        .post(&format!("{}/v1/symbol/find-definition", BASE_URL))
        .json(&json!({
            "position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_source_code": false,
            "include_raw_response": false
        }))
        .send()
        .await?;

    // Request should complete successfully
    assert!(response.status().is_success() || response.status().is_client_error());

    // Verify the same container is still being used (no new containers spawned)
    let containers_after_request: Vec<String> = docker
        .list_containers(Some(options.clone()))
        .await?
        .iter()
        .filter_map(|c| c.id.clone())
        .collect();
    assert_eq!(
        containers_after_request.len(),
        1,
        "Expected same container to be reused"
    );
    assert_eq!(
        containers_after_request[0], initial_containers[0],
        "Expected same container ID"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_request_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Test find-definition endpoint
    let response = client
        .post(&format!("{}/v1/symbol/find-definition", BASE_URL))
        .json(&json!({
            "position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_source_code": false,
            "include_raw_response": false
        }))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await?;

    // Should have definitions field (even if empty)
    assert!(body.get("definitions").is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multiple_requests_same_container() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let docker = Docker::connect_with_socket_defaults()?;
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // First request - container already spawned during service startup
    let response1 = client
        .post(&format!("{}/v1/symbol/find-definition", BASE_URL))
        .json(&json!({
            "position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_source_code": false,
            "include_raw_response": false
        }))
        .send()
        .await?;

    assert!(response1.status().is_success());

    sleep(Duration::from_secs(2)).await;

    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec!["nuanced-lsp-python-".to_string()]);
    filters.insert("status".to_string(), vec!["running".to_string()]);
    let options = ListContainersOptions {
        filters,
        ..Default::default()
    };
    let containers_after_first: Vec<String> = docker
        .list_containers(Some(options.clone()))
        .await?
        .iter()
        .filter_map(|c| c.id.clone())
        .collect();
    let first_count = containers_after_first.len();
    assert_eq!(
        first_count, 1,
        "Expected exactly one Python container after first request"
    );

    // Second request - should reuse container
    let response2 = client
        .post(&format!("{}/v1/symbol/find-references", BASE_URL))
        .json(&json!({
            "identifier_position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_code_context_lines": 0
        }))
        .send()
        .await?;

    assert!(response2.status().is_success());

    sleep(Duration::from_secs(1)).await;
    let containers_after_second: Vec<String> = docker
        .list_containers(Some(options.clone()))
        .await?
        .iter()
        .filter_map(|c| c.id.clone())
        .collect();
    assert_eq!(
        containers_after_second.len(),
        first_count,
        "Expected same number of containers - should reuse existing container"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_files() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let client = Client::new();
    let response = client
        .get(&format!("{}/v1/workspace/list-files", BASE_URL))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await?;

    // The endpoint returns a direct array of filenames, not an object with a "files" field
    assert!(body.is_array());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_references() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let response = client
        .post(&format!("{}/v1/symbol/find-references", BASE_URL))
        .json(&json!({
            "identifier_position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_code_context_lines": 0
        }))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await?;

    // Should have references field
    assert!(body.get("references").is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_references_with_context_lines() -> Result<(), Box<dyn std::error::Error>> {
    get_fixture().await?;

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Test with include_code_context_lines = 3
    // Looking for references to "hello" function in test.py
    // test.py contains:
    // def hello():\n    return \"hello\"\n\ndef world():\n    return \"world\"\n\nmessage = hello()\n
    let response = client
        .post(&format!("{}/v1/symbol/find-references", BASE_URL))
        .json(&json!({
            "identifier_position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}  // "hello" function definition
            },
            "include_code_context_lines": 3
        }))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await?;

    // Verify references field exists
    assert!(body.get("references").is_some());
    let references = body["references"].as_array().unwrap();
    assert!(
        references.len() > 0,
        "Should find at least one reference to 'hello'"
    );

    // Verify that references with context have source_code field
    for reference in references {
        if let Some(source_code) = reference.get("source_code") {
            let source = source_code.as_str().unwrap();

            // Source code should not be empty when context_lines is provided
            assert!(
                !source.is_empty(),
                "Source code should not be empty with context_lines=3"
            );

            // Source code should contain multiple lines (definition + context)
            let line_count = source.lines().count();
            assert!(
                line_count > 1,
                "Expected multiple lines with context_lines=3, got {}",
                line_count
            );
        }
    }

    // Now test with include_code_context_lines = 0 and verify NO source code is included
    let response_no_context = client
        .post(&format!("{}/v1/symbol/find-references", BASE_URL))
        .json(&json!({
            "identifier_position": {
                "path": "test.py",
                "position": {"line": 0, "character": 4}
            },
            "include_code_context_lines": 0
        }))
        .send()
        .await?;

    assert!(response_no_context.status().is_success());
    let body_no_context: serde_json::Value = response_no_context.json().await?;

    let references_no_context = body_no_context["references"].as_array().unwrap();
    for reference in references_no_context {
        // With context_lines=0, source_code field should be absent or empty
        if let Some(source_code) = reference.get("source_code") {
            let source = source_code.as_str().unwrap_or("");
            assert!(
                source.is_empty(),
                "Source code should be empty with context_lines=0"
            );
        }
    }

    Ok(())
}

/// Final test that cleans up the shared fixture
/// Named with zzz prefix to run last (tests run alphabetically within serial group)
#[tokio::test]
#[serial]
async fn test_zzz_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    cleanup_fixture().await?;
    Ok(())
}
