use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use log::{error, info};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

mod lsp_process;
use lsp_process::{JsonRpcMessage, LspProcess};

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
struct AppState {
    lsp_process: Arc<Mutex<LspProcess>>,
}

/// Health check endpoint - simple version that just returns OK
async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

/// LSP JSON-RPC endpoint
/// Accepts LSP requests and forwards them to the LSP server process
async fn lsp_handler(
    data: web::Data<AppState>,
    request: web::Json<JsonRpcMessage>,
) -> impl Responder {
    let mut lsp = data.lsp_process.lock().await;

    match lsp.send_request(&request.0).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("LSP request failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("LSP error: {}", e),
            })
        }
    }
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
        Ok(process) => Arc::new(Mutex::new(process)),
        Err(e) => {
            error!("Failed to start LSP server: {}", e);
            return Err(e);
        }
    };

    info!("LSP server started successfully");

    let app_state = web::Data::new(AppState { lsp_process });

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/health", web::get().to(health))
            .route("/lsp", web::post().to(lsp_handler))
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}