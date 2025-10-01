use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use log::{error, info};
use std::sync::Arc;

mod api_types;
mod ast_grep;
mod handlers;
mod lsp;
mod manager;
mod utils;

use lsp::client::LspClient;
use lsp::languages::JediClient;
use lsp::process::ProcessHandler;
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

    // Convert Vec<String> to Vec<&str> for process spawning
    let lsp_args_refs: Vec<&str> = args.lsp_args.iter().map(|s| s.as_str()).collect();

    // Start the LSP server process and create client
    let child = tokio::process::Command::new(&args.lsp_command)
        .args(&lsp_args_refs)
        .current_dir(&args.workspace_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn LSP server process: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;

    let process_handler = ProcessHandler::new(child).await.map_err(|e| {
        error!("Failed to create process handler: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Create JediClient (hardcoded for Python container, but manager abstracts over all clients)
    let mut jedi_client = JediClient::new(process_handler, args.workspace_path.clone());

    // Initialize the LSP server
    jedi_client.initialize(args.workspace_path.clone()).await.map_err(|e| {
        error!("Failed to initialize LSP server: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    info!("LSP server started and initialized successfully");

    let manager = Manager::new(
        Arc::new(tokio::sync::Mutex::new(Box::new(jedi_client) as Box<dyn lsp::client::LspClient>)),
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
            .route("/symbol/definitions-in-file", web::post().to(handlers::definitions_in_file::definitions_in_file))
            .route("/workspace/list-files", web::get().to(handlers::list_files::list_files))
            .route("/workspace/read-source-code", web::post().to(handlers::read_source_code::read_source_code))
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}
