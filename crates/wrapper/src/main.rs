use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use log::{error, info, warn};
use std::sync::Arc;

mod handlers;
mod lsp;
mod manager;

use lsp::client::LspClient;
use lsp::languages::{GenericLspClient, GoplsClient};
use lsp::process::ProcessHandler;
use manager::Manager;
use lsproxy_common::utils::workspace_documents::{
    DidOpenConfiguration, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS, RUBY_FILE_PATTERNS,
    TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS, RUST_FILE_PATTERNS, GOLANG_FILE_PATTERNS,
    JAVA_FILE_PATTERNS, C_AND_CPP_FILE_PATTERNS, CSHARP_FILE_PATTERNS,
};

/// HTTP wrapper for LSP servers
/// Provides HTTP endpoints for LSP JSON-RPC communication
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The LSP server command to run (e.g., "gopls", "ruby-lsp", "jedi-language-server")
    #[arg(long)]
    lsp_command: String,

    /// Arguments to pass to the LSP server (e.g., "--use-launcher", "-v")
    /// Can be specified multiple times: --lsp-arg --use-launcher --lsp-arg -v
    #[arg(long = "lsp-arg")]
    lsp_args: Vec<String>,

    /// The workspace path (mounted in container, consistent with main LSProxy)
    #[arg(long, default_value = "/mnt/workspace")]
    workspace_path: String,

    /// The port to listen on
    #[arg(long, default_value = "8080")]
    port: u16,
}

/// Application state shared across handlers
pub struct AppState {
    pub manager: Manager,
}

/// Health check endpoint - simple version that just returns OK
async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let args = Args::parse();

    info!("Starting LSP wrapper for: {}", args.lsp_command);
    if !args.lsp_args.is_empty() {
        info!("  with args: {:?}", args.lsp_args);
    }
    info!("Workspace path: {}", args.workspace_path);
    info!("Listening on port: {}", args.port);

    // Convert Vec<String> to Vec<&str> for process spawning
    let lsp_args_refs: Vec<&str> = args.lsp_args.iter().map(|s| s.as_str()).collect();

    // Start the LSP server process and create client
    let child = tokio::process::Command::new(&args.lsp_command)
        .args(&lsp_args_refs)
        .current_dir(&args.workspace_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit()) // Inherit stderr so it shows in docker logs
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn LSP server process: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

    let process_handler = ProcessHandler::new(child).await.map_err(|e| {
        error!("Failed to create process handler: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Get language from LSP_LANGUAGE environment variable (required)
    let language = std::env::var("LSP_LANGUAGE").map_err(|_| {
        error!("LSP_LANGUAGE environment variable is not set");
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "LSP_LANGUAGE environment variable is required but not set",
        )
    })?;

    info!("Language detected: {}", language);

    // Configure based on language
    let (file_patterns, did_open_config) = match language.as_str() {
        "php" => (PHP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy),
        "python" => (PYTHON_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "ruby" => (RUBY_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "typescript" | "javascript" => (TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy),
        "rust" => (RUST_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "go" | "golang" => (GOLANG_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "java" => (JAVA_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "cpp" | "c" => (C_AND_CPP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy),
        "csharp" => (CSHARP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        _ => {
            error!("Unknown language '{}'. Supported languages: php, python, ruby, typescript, javascript, rust, go, golang, java, cpp, c, csharp", language);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unsupported language: {}", language),
            ));
        }
    };

    // Create base client
    let base_client = GenericLspClient::new(
        process_handler,
        args.workspace_path.clone(),
        file_patterns.iter().map(|&s| s.to_string()).collect(),
        did_open_config,
    );

    // Apply language-specific initialization and setup
    let mut client: Box<dyn LspClient> = match language.as_str() {
        "go" => {
            info!("Configuring Go with custom workspace folder detection (go.work/go.mod)");
            // Convert GenericLspClient components to GoplsClient
            let (process, json_rpc, workspace_documents, pending_requests) = base_client.into_components();
            let gopls_client = GoplsClient::new(
                process,
                json_rpc,
                workspace_documents,
                pending_requests,
            );
            Box::new(gopls_client)
        }
        "rust" => {
            info!("Configuring Rust with initialization options and setup workspace");
            let configured_client = base_client
                .with_initialization_options(serde_json::json!({
                    "cargo": {
                        "sysroot": serde_json::Value::Null
                    }
                }))
                .with_setup_workspace_method("rust-analyzer/reloadWorkspace".to_string());
            Box::new(configured_client)
        }
        "cpp" | "c" => {
            info!("Configuring C/C++ with clangd initialization options");
            let configured_client = base_client
                .with_initialization_options(serde_json::json!({
                    "clangdFileStatus": true
                }));
            Box::new(configured_client)
        }
        _ => Box::new(base_client),
    };

    // Initialize the LSP server
    client.initialize(args.workspace_path.clone()).await.map_err(|e| {
        error!("Failed to initialize LSP server: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Java-specific: Wait for ServiceReady notification
    if language.as_str() == "java" {
        use lsp::ExpectedMessageKey;
        info!("Java: waiting for ServiceReady notification. This may take up to 3 minutes...");

        let mut notification_rx = client
            .get_pending_requests()
            .add_notification(ExpectedMessageKey {
                method: "language/status".to_string(),
                params: serde_json::json!({
                    "type": "ServiceReady",
                    "message": "ServiceReady"
                }),
            })
            .await
            .map_err(|e| {
                error!("Failed to add ServiceReady notification listener: {}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;

        tokio::time::timeout(
            std::time::Duration::from_secs(180),
            notification_rx.recv()
        )
        .await
        .map_err(|_| {
            error!("Timeout waiting for Java ServiceReady notification");
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Timeout waiting for Java ServiceReady notification after 180 seconds",
            )
        })?
        .map_err(|e| {
            error!("Error receiving ServiceReady notification: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

        info!("Java: ServiceReady notification received!");
    }

    // Setup workspace (e.g., rust-analyzer/reloadWorkspace)
    client.setup_workspace(&args.workspace_path).await.map_err(|e| {
        error!("Failed to setup workspace: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    info!("LSP server started and initialized successfully");

    let manager = Manager::new(
        Arc::new(tokio::sync::Mutex::new(client)),
        args.workspace_path.clone(),
    );

    let app_state = web::Data::new(AppState { manager });

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/health", web::get().to(health))
            .route("/symbol/find-identifier", web::post().to(handlers::find_identifier::find_identifier))
            .route("/symbol/find-definition", web::post().to(handlers::find_definition::find_definition))
            .route("/symbol/find-references", web::post().to(handlers::find_references::find_references))
            .route("/symbol/find-referenced-symbols", web::post().to(handlers::find_referenced_symbols::find_referenced_symbols))
            .route("/symbol/definitions-in-file", web::get().to(handlers::definitions_in_file::definitions_in_file))
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}
