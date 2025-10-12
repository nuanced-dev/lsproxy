/// Integration test for Python language support in the new containerized architecture.
///
/// This test:
/// - Starts the LSProxy service in-process (not in Docker)
/// - The service spawns a Python language container on-demand
/// - Tests workspace and symbol endpoints
/// - Requires: lsproxy-python:latest Docker image
use lsproxy::api_types::{
    set_global_mount_dir, FilePosition, FileRange, HealthResponse, Position, Range, Symbol,
    SymbolResponse,
};
use lsproxy::{initialize_app_state, run_server};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn wait_for_server(base_url: &str) {
    let client = reqwest::blocking::Client::new();
    let health_url = format!("{}/v1/system/health", base_url);

    for _ in 0..60 {
        // Try for 60 seconds (container spawn can take time)
        if let Ok(response) = client.get(&health_url).send() {
            if let Ok(health) = response.json::<HealthResponse>() {
                if health.status == "ok" {
                    println!("Server is healthy");
                    return;
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!("Server did not respond with healthy status within 60 seconds");
}

#[test]
fn test_server_integration_python() -> Result<(), Box<dyn std::error::Error>> {
    // Use the sample project directory as the workspace
    // This is the path on the host that will be mounted into language containers
    let mount_dir = "/mnt/lsproxy_root/sample_project/python";

    let (tx, rx) = mpsc::channel();

    // Spawn the server in a separate thread
    let _server_thread = thread::spawn(move || {
        // Configure for new containerized architecture
        std::env::set_var("USE_AUTH", "false");
        std::env::set_var("HOST_WORKSPACE_PATH", mount_dir); // For spawning containers
        std::env::set_var("RUST_LOG", "info");
        set_global_mount_dir(mount_dir); // For workspace operations

        let system = actix_web::rt::System::new();
        if let Err(e) = system.block_on(async {
            match initialize_app_state().await {
                Ok(app_state) => run_server(app_state).await,
                Err(e) => {
                    tx.send(format!("Failed to initialize app state: {}", e))
                        .unwrap();
                    Ok(())
                }
            }
        }) {
            tx.send(format!("System error: {}", e)).unwrap();
        }
    });

    // Give the server time to start and spawn language container
    thread::sleep(Duration::from_secs(10));

    // Check for any errors from the server thread
    if let Ok(error_msg) = rx.try_recv() {
        return Err(error_msg.into());
    }

    let base_url = "http://localhost:4444";
    wait_for_server(base_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build client");

    // Test workspace/list-files endpoint (language-agnostic, handled by base service)
    println!("Testing list-files endpoint...");
    let response = client
        .get(format!("{}/v1/workspace/list-files", base_url))
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);

    let mut workspace_files: Vec<String> = response.json().expect("Failed to parse JSON");

    // Check if the expected files are present
    let mut expected_files = vec![
        "decorators.py",
        "graph.py",
        "main.py",
        "search.py",
        "__init__.py",
    ];
    assert_eq!(
        workspace_files.len(),
        expected_files.len(),
        "Unexpected number of files"
    );

    workspace_files.sort();
    expected_files.sort();
    assert_eq!(workspace_files, expected_files, "File lists do not match");

    // Test definitions-in-file endpoint (requires Python language container)
    println!("Testing definitions-in-file endpoint...");
    let response = client
        .get(format!("{}/v1/symbol/definitions-in-file", base_url))
        .query(&[("file_path", "main.py")])
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let returned_symbols: SymbolResponse =
        serde_json::from_value(response.json().expect("Failed to parse JSON"))?;
    let expected = vec![
        Symbol {
            name: String::from("plot_path"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 6,
                    character: 4,
                },
            },
            file_range: FileRange {
                path: String::from("main.py"),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 12,
                        character: 14,
                    },
                },
            },
        },
        Symbol {
            name: String::from("main"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 14,
                    character: 4,
                },
            },
            file_range: FileRange {
                path: String::from("main.py"),
                range: Range {
                    start: Position {
                        line: 14,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 28,
                    },
                },
            },
        },
    ];
    assert_eq!(returned_symbols, expected);
    Ok(())
}
