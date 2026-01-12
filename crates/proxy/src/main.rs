use clap::Parser;

use log::{error, info};
use proxy::{
    initialize_app_state_with_mount_dir, run_server_with_port_and_host, write_openapi_to_file,
};
use std::path::PathBuf;

/// Command line interface for Nuanced LSP server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI specification to openapi.json file
    #[arg(short, long)]
    write_openapi: bool,

    /// Host address to bind the server to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Override the default mount directory path where your workspace files are located
    #[arg(long)]
    mount_dir: Option<String>,

    /// Port number to bind the server to
    #[arg(long, default_value_t = 4444)]
    port: u16,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up panic handler for better error reporting
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Server panicked: {:?}", panic_info);
    }));

    // Initialize tracing subscriber for better logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle OpenAPI spec generation if requested
    if cli.write_openapi {
        if let Err(e) = write_openapi_to_file(&PathBuf::from("openapi.json")) {
            error!("Error: Failed to write the openapi.json to a file. Please see error for more details.");
            return Err(e);
        }
        return Ok(());
    }

    // Initialize application state with optional mount directory override
    let app_state = initialize_app_state_with_mount_dir(cli.mount_dir.as_deref())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Get orchestrator reference for shutdown handling
    let orchestrator = app_state.orchestrator();

    // Run the server with specified host
    info!("Starting on port {}", cli.port);

    // Run the server and wait for it to complete
    let server_result = run_server_with_port_and_host(app_state, cli.port, &cli.host).await;

    // After server stops (normal shutdown or error), cleanup all containers
    // Note: This won't run if the process panics - that's what the watchdog is for
    info!("Server stopped, cleaning up containers...");

    if let Err(e) = orchestrator.cleanup_all().await {
        error!("Error during container cleanup: {}", e);
        // Watchdog will perform emergency cleanup when parent stops
    } else {
        info!("All containers cleaned up successfully");
        // Watchdog will see no containers to clean and exit gracefully
    }

    // Stop the watchdog now that we're done
    if let Err(e) = orchestrator.stop_watchdog_container().await {
        error!("Error stopping watchdog: {}", e);
    } else {
        info!("Watchdog stopped");
    }

    server_result
}
