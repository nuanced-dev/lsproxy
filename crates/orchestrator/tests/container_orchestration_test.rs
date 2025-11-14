/// Integration tests for containerized LSProxy architecture
///
/// Tests the full flow:
/// 1. Base LSProxy service running in container
/// 2. Dynamic spawning of language-specific containers (Python)
/// 3. Request forwarding and response handling
/// 4. Container lifecycle management
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StopContainerOptions,
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use reqwest::Client;
use serde_json::json;
use serial_test::serial;
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

const BASE_IMAGE: &str = "lsproxy-service:latest";
const PYTHON_IMAGE: &str = "lsproxy-python:latest";
const SERVICE_PORT: u16 = 14444; // Use non-standard port to avoid conflicts
const CONTAINER_PORT: u16 = 4444; // Port the service listens on inside container
const BASE_URL: &str = "http://localhost:14444";
const MAX_RETRIES: u32 = 30;
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Test fixture that manages container lifecycle
struct ContainerFixture {
    docker: Docker,
    service_container_id: Option<String>,
    workspace_dir: TempDir,
}

impl ContainerFixture {
    /// Create new test fixture with workspace
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let docker = Docker::connect_with_socket_defaults()?;

        // Clean up any existing test container from previous runs
        let _ = docker
            .remove_container(
                "lsproxy-test-service",
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        // Verify required images exist
        Self::verify_images(&docker).await?;

        let workspace_dir = tempfile::tempdir()?;

        // Create test Python files
        Self::create_test_files(&workspace_dir)?;

        Ok(Self {
            docker,
            service_container_id: None,
            workspace_dir,
        })
    }

    /// Verify required Docker images are available
    async fn verify_images(docker: &Docker) -> Result<(), Box<dyn std::error::Error>> {
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![BASE_IMAGE.to_string()]);

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = docker.list_images(Some(options)).await?;
        if images.is_empty() {
            return Err(format!("Required image {} not found. Run: docker build -f dockerfiles/service.Dockerfile -t {} .", BASE_IMAGE, BASE_IMAGE).into());
        }

        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![PYTHON_IMAGE.to_string()]);

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = docker.list_images(Some(options)).await?;
        if images.is_empty() {
            return Err(format!("Required image {} not found. Run: docker build -f dockerfiles/python.Dockerfile -t {} .", PYTHON_IMAGE, PYTHON_IMAGE).into());
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

    /// Start the base LSProxy service container
    async fn start_service(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_path = self
            .workspace_dir
            .path()
            .to_str()
            .ok_or("Invalid workspace path")?;

        let config = Config {
            image: Some(BASE_IMAGE),
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
            name: "lsproxy-test-service",
            ..Default::default()
        };

        let container = self.docker.create_container(Some(options), config).await?;
        self.service_container_id = Some(container.id.clone());

        self.docker
            .start_container::<String>(&container.id, None)
            .await?;

        // Wait for service to be healthy
        self.wait_for_health().await?;

        Ok(())
    }

    /// Wait for service health check to pass
    async fn wait_for_health(&self) -> Result<(), Box<dyn std::error::Error>> {
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

    /// Get list of Python containers spawned by the service
    async fn get_python_containers(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec!["lsproxy-python-".to_string()]);
        filters.insert("status".to_string(), vec!["running".to_string()]);

        let options = ListContainersOptions {
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;
        Ok(containers.iter().filter_map(|c| c.id.clone()).collect())
    }

    /// Clean up all test containers
    async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Stop and remove spawned Python containers
        let python_containers = self.get_python_containers().await.unwrap_or_default();
        for container_id in python_containers {
            let _ = self
                .docker
                .stop_container(&container_id, Some(StopContainerOptions { t: 5 }))
                .await;
            let _ = self
                .docker
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;
        }

        // Stop and remove service container
        if let Some(container_id) = self.service_container_id.take() {
            let _ = self
                .docker
                .stop_container(&container_id, Some(StopContainerOptions { t: 5 }))
                .await;
            let _ = self
                .docker
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;
        }

        Ok(())
    }
}

impl Drop for ContainerFixture {
    fn drop(&mut self) {
        // Best effort cleanup
        if let Some(container_id) = &self.service_container_id {
            let docker = self.docker.clone();
            let container_id = container_id.clone();
            tokio::spawn(async move {
                let _ = docker
                    .stop_container(&container_id, Some(StopContainerOptions { t: 2 }))
                    .await;
                let _ = docker
                    .remove_container(
                        &container_id,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            });
        }
    }
}

#[tokio::test]
#[serial]
async fn test_service_health() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

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

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_container_spawn_on_request() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

    // With eager initialization, Python container should be spawned during service startup
    let initial_containers = fixture.get_python_containers().await?;
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
    let containers_after_request = fixture.get_python_containers().await?;
    assert_eq!(
        containers_after_request.len(),
        1,
        "Expected same container to be reused"
    );
    assert_eq!(
        containers_after_request[0], initial_containers[0],
        "Expected same container ID"
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_request_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

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

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multiple_requests_same_container() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // First request - spawns container
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
    let containers_after_first = fixture.get_python_containers().await?;
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
    let containers_after_second = fixture.get_python_containers().await?;
    assert_eq!(
        containers_after_second.len(),
        first_count,
        "Expected same number of containers - should reuse existing container"
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

    let client = Client::new();
    let response = client
        .get(&format!("{}/v1/workspace/list-files", BASE_URL))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await?;

    // The endpoint returns a direct array of filenames, not an object with a "files" field
    assert!(body.is_array());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

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

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_references_with_context_lines() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = ContainerFixture::new().await?;
    fixture.start_service().await?;

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
    assert!(references.len() > 0, "Should find at least one reference to 'hello'");

    // Verify that references with context have source_code field
    for reference in references {
        if let Some(source_code) = reference.get("source_code") {
            let source = source_code.as_str().unwrap();

            // Source code should not be empty when context_lines is provided
            assert!(!source.is_empty(), "Source code should not be empty with context_lines=3");

            // Source code should contain multiple lines (definition + context)
            let line_count = source.lines().count();
            assert!(line_count > 1, "Expected multiple lines with context_lines=3, got {}", line_count);
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
            assert!(source.is_empty(), "Source code should be empty with context_lines=0");
        }
    }

    fixture.cleanup().await?;
    Ok(())
}
