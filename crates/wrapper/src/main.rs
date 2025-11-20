use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use log::{error, info, warn};
use std::sync::Arc;

const DEFAULT_RBENV_ROOT: &str = "/opt/rbenv";

mod handlers;
mod lsp;
mod manager;

use lsp::client::LspClient;
use lsp::languages::{GenericLspClient, GoplsClient, SorbetClient};
use lsp::process::ProcessHandler;
use lsproxy_common::utils::workspace_documents::{
    DidOpenConfiguration, CSHARP_FILE_PATTERNS, C_AND_CPP_FILE_PATTERNS, GOLANG_FILE_PATTERNS,
    JAVA_FILE_PATTERNS, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS, RUBY_FILE_PATTERNS,
    RUST_FILE_PATTERNS, TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS,
};
use manager::Manager;

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

    // Get language from LSP_LANGUAGE environment variable (required)
    let language = std::env::var("LSP_LANGUAGE").map_err(|_| {
        error!("LSP_LANGUAGE environment variable is not set");
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "LSP_LANGUAGE environment variable is required but not set",
        )
    })?;

    info!("Language detected: {}", language);

    // Ensure Sorbet version matches project expectations before spawning LSP
    let mut lsp_args = args.lsp_args.clone();

    // Optional RBENV_VERSION override to handle .ruby-version that isn't installed
    let mut rbenv_version_override: Option<String> = None;
    if language == "ruby" || language == "ruby-sorbet" {
        let rbenv_root =
            std::env::var("RBENV_ROOT").unwrap_or_else(|_| DEFAULT_RBENV_ROOT.to_string());
        let desired_version_path = std::path::Path::new(&args.workspace_path).join(".ruby-version");
        if let Ok(contents) = std::fs::read_to_string(&desired_version_path) {
            let desired = contents.trim();
            if !desired.is_empty() {
                let installed = std::path::Path::new(&rbenv_root)
                    .join("versions")
                    .join(desired);
                if installed.exists() {
                    info!("Using Ruby version from .ruby-version: {}", desired);
                    rbenv_version_override = Some(desired.to_string());
                } else {
                    warn!(
                        "Ruby version {} from .ruby-version is not installed; falling back to global",
                        desired
                    );
                }
            }
        }

        if rbenv_version_override.is_none() {
            let global_path = std::path::Path::new(&rbenv_root).join("version");
            if let Ok(contents) = std::fs::read_to_string(&global_path) {
                let global = contents.trim();
                if !global.is_empty() {
                    info!("Using global Ruby version: {}", global);
                    rbenv_version_override = Some(global.to_string());
                }
            }
        }
    }

    if language == "ruby-sorbet" {
        match crate::lsp::languages::sorbet::ensure_sorbet_version(&args.workspace_path).await {
            Ok(Some(ver)) => {
                // Force Sorbet to run with the detected version when multiple are installed
                lsp_args.insert(0, format!("_{}_", ver));
            }
            Ok(None) => {
                // No version specified; use whatever is baked/preinstalled
            }
            Err(e) => {
                warn!("Unable to ensure Sorbet version: {}", e);
            }
        }
    }

    // Start the LSP server process and create client
    // Create a debug log file for LSP stderr output
    let log_file_path = format!("/tmp/{}.log", args.lsp_command);
    let stderr_file = std::fs::File::create(&log_file_path).map_err(|e| {
        error!("Failed to create debug log file {}: {}", log_file_path, e);
        e
    })?;
    info!("LSP stderr will be logged to: {}", log_file_path);

    let mut cmd = tokio::process::Command::new(&args.lsp_command);
    cmd.args(&lsp_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .current_dir(&args.workspace_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::from(stderr_file));

    if let Some(ver) = rbenv_version_override {
        cmd.env("RBENV_VERSION", ver);
    }

    let child = cmd.spawn().map_err(|e| {
        error!("Failed to spawn LSP server process: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let process_handler = ProcessHandler::new(child).await.map_err(|e| {
        error!("Failed to create process handler: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Configure based on language
    let (file_patterns, did_open_config) = match language.as_str() {
        "php" => (PHP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy),
        "python" => (PYTHON_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "ruby" => (RUBY_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "ruby-sorbet" => {
            info!("Detected ruby-sorbet language - using Lazy didOpen configuration");
            (RUBY_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy)
        }
        "typescript" | "javascript" => (
            TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS.to_vec(),
            DidOpenConfiguration::Lazy,
        ),
        "rust" => (RUST_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "go" => (GOLANG_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "java" => (JAVA_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        "cpp" | "c" => (C_AND_CPP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::Lazy),
        "csharp" => (CSHARP_FILE_PATTERNS.to_vec(), DidOpenConfiguration::None),
        _ => {
            error!("Unknown language '{}'. Supported languages: php, python, ruby, ruby-sorbet, typescript, javascript, rust, go, java, cpp, c, csharp", language);
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
    info!(
        "Checking LSP command for language-specific configuration: '{}'",
        args.lsp_command
    );
    let mut client: Box<dyn LspClient> = match args.lsp_command.as_str() {
        "srb" => {
            info!("Configuring Sorbet with custom workspace folder detection (sorbet/config)");
            // Convert GenericLspClient components to SorbetClient
            let (process, json_rpc, workspace_documents, pending_requests) =
                base_client.into_components();
            let sorbet_client =
                SorbetClient::new(process, json_rpc, workspace_documents, pending_requests);
            Box::new(sorbet_client)
        }
        _ => match language.as_str() {
            "go" => {
                info!("Configuring Go with custom workspace folder detection (go.work/go.mod)");
                // Convert GenericLspClient components to GoplsClient
                let (process, json_rpc, workspace_documents, pending_requests) =
                    base_client.into_components();
                let gopls_client =
                    GoplsClient::new(process, json_rpc, workspace_documents, pending_requests);
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
                let configured_client =
                    base_client.with_initialization_options(serde_json::json!({
                        "clangdFileStatus": true
                    }));
                Box::new(configured_client)
            }
            _ => Box::new(base_client),
        },
    };

    // Initialize the LSP server
    client
        .initialize(args.workspace_path.clone())
        .await
        .map_err(|e| {
            error!("Failed to initialize LSP server: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

    // Java-specific: Wait for ServiceReady notification
    if language.as_str() == "java" {
        use lsp::ExpectedMessageKey;
        info!("Java: waiting for ServiceReady notification (no timeout - caller controls overall timeout)...");

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

        // Wait indefinitely for ServiceReady notification
        // The orchestrator health check and CLI timeout control overall timing
        notification_rx.recv().await.map_err(|e| {
            error!("Error receiving ServiceReady notification: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

        info!("Java: ServiceReady notification received!");
    }

    // Setup workspace (e.g., rust-analyzer/reloadWorkspace)
    client
        .setup_workspace(&args.workspace_path)
        .await
        .map_err(|e| {
            error!("Failed to setup workspace: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

    info!("LSP server started and initialized successfully");

    let manager = Manager::new(Arc::new(tokio::sync::Mutex::new(client)));

    let app_state = web::Data::new(AppState { manager });

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/health", web::get().to(health))
            .route(
                "/symbol/find-identifier",
                web::post().to(handlers::find_identifier::find_identifier),
            )
            .route(
                "/symbol/find-definition",
                web::post().to(handlers::find_definition::find_definition),
            )
            .route(
                "/symbol/find-references",
                web::post().to(handlers::find_references::find_references),
            )
            .route(
                "/symbol/find-referenced-symbols",
                web::post().to(handlers::find_referenced_symbols::find_referenced_symbols),
            )
            .route(
                "/symbol/definitions-in-file",
                web::get().to(handlers::definitions_in_file::definitions_in_file),
            )
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}
