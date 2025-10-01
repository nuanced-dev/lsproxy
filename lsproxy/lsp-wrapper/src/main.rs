use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use log::{error, info};
use std::sync::Arc;

mod api_types;
mod ast_grep;
mod handlers;
mod lsp_process;
mod manager;
mod utils;

use lsp_process::LspProcess;
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

    // Start the LSP server process
    let lsp_process = match LspProcess::new(&args.lsp_command, &lsp_args_refs, &args.workspace_path).await {
        Ok(process) => process,
        Err(e) => {
            error!("Failed to start LSP server: {}", e);
            return Err(e);
        }
    };

    info!("LSP server started successfully");

    let manager = Manager::new(Arc::new(lsp_process), args.workspace_path.clone());

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
            .route("/file/list-files", web::get().to(handlers::list_files::list_files))
            .route("/file/read-source", web::post().to(handlers::read_source_code::read_source_code))
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}
