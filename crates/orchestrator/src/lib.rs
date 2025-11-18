use actix_cors::Cors;
mod middleware;
use actix_web::{
    web::{get, post, resource, scope, Data},
    App, HttpServer,
};
use handlers::{find_identifier, read_source_code};
use log::{error, info, warn};
use lsproxy_common::api_types::{FindIdentifierRequest, IdentifierResponse};
use middleware::JwtMiddleware;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Local modules
pub mod container;
mod handlers;

use crate::handlers::{
    definitions_in_file, find_definition, find_referenced_symbols, find_references, health_check,
    list_files,
};
use lsproxy_common::api_types::{
    get_mount_dir, set_global_mount_dir, CodeContext, DefinitionResponse, ErrorResponse,
    FilePosition, FileRange, FileSymbolsRequest, GetDefinitionRequest, GetReferencedSymbolsRequest,
    GetReferencesRequest, HealthResponse, Position, ReferenceWithSymbolDefinitions,
    ReferencedSymbolsResponse, ReferencesResponse, SupportedLanguages, Symbol, SymbolResponse,
};
// use lsproxy_common::utils::doc_utils::make_code_sample;

pub fn check_mount_dir() -> std::io::Result<()> {
    fs::read_dir(get_mount_dir())?;
    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "lsproxy",
        version = "0.2.1",
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    components(
        schemas(
            FileSymbolsRequest,
            GetDefinitionRequest,
            GetReferencesRequest,
            GetReferencedSymbolsRequest,
            SupportedLanguages,
            DefinitionResponse,
            ReferencesResponse,
            ReferencedSymbolsResponse,
            SymbolResponse,
            ReferenceWithSymbolDefinitions,
            FilePosition,
            Position,
            Symbol,
            ErrorResponse,
            CodeContext,
            FileRange,
            HealthResponse,
            FindIdentifierRequest,
            IdentifierResponse,
        )
    ),
    paths(
        crate::handlers::definitions_in_file,
        crate::handlers::find_definition,
        crate::handlers::find_references,
        crate::handlers::health_check,
        crate::handlers::list_files,
        crate::handlers::read_source_code,
        crate::handlers::find_referenced_symbols,
        crate::handlers::find_identifier,
    ),
    tags(
        (name = "lsproxy-api", description = "LSP Proxy API")
    ),
    servers(
        (url = "http://localhost:4444/v1", description = "API server v1")
    )
)]
pub struct ApiDoc;

pub struct AppState {
    orchestrator: Arc<container::ContainerOrchestrator>,
    workspace_path: String,
}

impl AppState {
    /// Get a reference to the orchestrator for shutdown handling
    pub fn orchestrator(&self) -> Arc<container::ContainerOrchestrator> {
        Arc::clone(&self.orchestrator)
    }
}

pub async fn initialize_app_state() -> Result<Data<AppState>, Box<dyn std::error::Error>> {
    initialize_app_state_with_mount_dir(None).await
}

pub async fn initialize_app_state_with_mount_dir(
    mount_dir_override: Option<&str>,
) -> Result<Data<AppState>, Box<dyn std::error::Error>> {
    if let Some(global_mount_dir) = mount_dir_override {
        set_global_mount_dir(global_mount_dir);
        warn!("Changing global mount dir to: {}", global_mount_dir);
    }

    if check_mount_dir().is_err() {
        error!(
            "Your workspace isn't mounted at '{}'. Please mount your workspace at this location.",
            get_mount_dir().to_string_lossy()
        );
        std::process::exit(1);
    }

    let mount_dir_path = get_mount_dir();
    let workspace_path = mount_dir_path.to_string_lossy().to_string();

    // Initialize container orchestrator
    let orchestrator = Arc::new(container::ContainerOrchestrator::new().await?);

    // Spawn watchdog container to monitor this service
    // The watchdog will cleanup language containers if this service dies unexpectedly
    info!("Spawning watchdog container...");
    if let Err(e) = orchestrator.spawn_watchdog().await {
        warn!("Failed to spawn watchdog (will continue without it): {}", e);
    }

    // Initialize workspace: detect languages and spawn containers upfront
    // This matches the original Manager::start_langservers() behavior
    info!("Initializing workspace and spawning language containers...");
    orchestrator.initialize_workspace(&workspace_path).await?;
    info!("Workspace initialization complete");

    Ok(Data::new(AppState {
        orchestrator,
        workspace_path,
    }))
}

// Helper enum for cleaner matching
#[derive(Debug)]
enum Method {
    Get,
    Post,
}

pub async fn run_server(app_state: Data<AppState>) -> std::io::Result<()> {
    run_server_with_host(app_state, "0.0.0.0").await
}

pub async fn run_server_with_host(app_state: Data<AppState>, host: &str) -> std::io::Result<()> {
    run_server_with_port_and_host(app_state, 4444, host).await
}

pub async fn run_server_with_port(app_state: Data<AppState>, port: u16) -> std::io::Result<()> {
    run_server_with_port_and_host(app_state, port, "0.0.0.0").await
}

pub async fn run_server_with_port_and_host(
    app_state: Data<AppState>,
    port: u16,
    host: &str,
) -> std::io::Result<()> {
    let mut openapi = ApiDoc::openapi();

    // Create components if none exist
    if openapi.components.is_none() {
        openapi.components = Some(utoipa::openapi::Components::default());
    }

    // Now we can safely unwrap and modify
    if let Some(components) = openapi.components.as_mut() {
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }

    // Parse the full server URL to get just the path component
    let server_path = openapi
        .servers
        .as_ref() // Get reference to the Option<Vec>
        .and_then(|servers| servers.first()) // Get first server if vec is not empty
        .and_then(|s| url::Url::parse(&s.url).ok())
        .map(|url| url.path().to_string()) // Convert path to owned String
        .and_then(|path| path.strip_prefix('/').map(|s| s.to_string())) // Convert stripped result to String
        .unwrap_or_else(String::new); // Use empty string as default

    // Initialize JWT middleware once before creating workers to fail fast
    // If this panics, it happens in the main thread before any workers start
    let jwt_middleware = if middleware::is_auth_enabled() {
        match JwtMiddleware::from_env() {
            Ok(middleware) => Some(middleware),
            Err(e) => {
                error!("Failed to initialize JWT middleware: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    HttpServer::new(move || {
        let mut api_scope = scope(format!("/{}", server_path).as_str());

        // Add routes based on OpenAPI paths
        for (path, path_item) in openapi.paths.paths.iter() {
            let method = if path_item.get.is_some() {
                Some(Method::Get)
            } else if path_item.post.is_some() {
                Some(Method::Post)
            } else {
                None
            };

            api_scope = match (path.as_str(), method) {
                ("/symbol/find-definition", Some(Method::Post)) =>
                    api_scope.service(resource(path).route(post().to(find_definition))),
                ("/symbol/find-references", Some(Method::Post)) =>
                    api_scope.service(resource(path).route(post().to(find_references))),
                ("/symbol/find-referenced-symbols", Some(Method::Post)) =>
                    api_scope.service(resource(path).route(post().to(find_referenced_symbols))),
                ("/symbol/find-identifier", Some(Method::Post)) =>
                    api_scope.service(resource(path).route(post().to(find_identifier))),
                ("/symbol/definitions-in-file", Some(Method::Get)) =>
                    api_scope.service(resource(path).route(get().to(definitions_in_file))),
                ("/workspace/list-files", Some(Method::Get)) =>
                    api_scope.service(resource(path).route(get().to(list_files))),
                ("/workspace/read-source-code", Some(Method::Post)) =>
                    api_scope.service(resource(path).route(post().to(read_source_code))),
                ("/system/health", Some(Method::Get)) =>
                    api_scope.service(resource(path).route(get().to(health_check))),
                (p, m) => panic!(
                    "Invalid path configuration for {}: {:?}. Ensure the OpenAPI spec matches your handlers.",
                    p,
                    m
                )
            };
        }

        App::new()
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .configure(|cfg| {
                if let Some(ref middleware) = jwt_middleware {
                    cfg.service(api_scope.wrap(middleware.clone()));
                } else {
                    cfg.service(api_scope);
                }
            })
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi.clone())
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

// const PYTHON_SAMPLE: &str = r#"
// import requests

// def get_pet(pet_id: int):
//     response = requests.get(f'/pets/{pet_id}')
//     return response.json()
// "#;

pub fn write_openapi_to_file(file_path: &PathBuf) -> std::io::Result<()> {
    // We use a clone since we're just adding the docs and writing it to the file. We don't need
    // this for runtime
    let mut openapi = ApiDoc::openapi().clone();
    // if let Some(path_item) = openapi.paths.paths.get_mut("/symbol/find-definition") {
    //     if let Some(post_op) = &mut path_item.post {
    //         let mut extensions = Extensions::default();
    //         extensions.insert(
    //             String::from("x-codeSamples"),
    //             serde_json::json!(vec![make_code_sample("python", PYTHON_SAMPLE),]),
    //         );
    //         post_op.extensions = Some(extensions);
    //     }
    // }

    // Create components if none exist
    if openapi.components.is_none() {
        openapi.components = Some(utoipa::openapi::Components::default());
    }

    // Now we can safely unwrap and modify
    if let Some(components) = openapi.components.as_mut() {
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
    let openapi_json =
        serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI to JSON");
    let mut file = File::create(file_path)?;
    file.write_all(openapi_json.as_bytes())?;
    info!("OpenAPI spec written to: {}", file_path.display());
    Ok(())
}

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::TempDir;

    fn simple_diff(text1: &str, text2: &str) -> String {
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        let mut diff = String::new();
        let mut diff_count = 0;

        for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
            if l1 != l2 {
                diff_count += 1;
                if diff_count <= 3 {
                    // Show up to 3 differences
                    diff.push_str(&format!("Line {}: '{}' vs '{}'\n", i + 1, l1, l2));
                }
            }
        }

        if lines1.len() != lines2.len() {
            diff.push_str(&format!(
                "Files have different number of lines: {} vs {}\n",
                lines1.len(),
                lines2.len()
            ));
        }

        if diff_count > 3 {
            diff.push_str(&format!("... and {} more differences\n", diff_count - 3));
        }

        diff
    }

    #[test]
    #[ignore] // Requires running ./scripts/generate_spec.sh to sync openapi.json
    fn test_openapi_json() -> Result<(), Box<dyn std::error::Error>> {
        // Create a new temporary directory
        let temp_dir = TempDir::new()?;
        let temp_openapi_path = temp_dir.path().join("openapi.json");

        // Write the OpenAPI spec to the temporary file
        write_openapi_to_file(&PathBuf::from(&temp_openapi_path))?;

        // Read the content of the generated file
        let generated_content = fs::read_to_string(&temp_openapi_path)
            .map_err(|e| format!("Failed to load generate openapi spec: {}", e))?;

        // Read the content of the existing file
        // Assume you have a known good file to compare against
        let existing_path = test_utils::workspace_root().join("openapi.json");
        let existing_content = fs::read_to_string(existing_path)
            .map_err(|e| format!("Failed to load existing openapi spec: {}", e))?;

        // Compare the contents
        if generated_content != existing_content {
            let diff = simple_diff(&existing_content, &generated_content);
            return Err(format!(
                "Differences: {}
                Generated OpenAPI JSON does not match existing content. Make sure to run ./scripts/generate_spec.sh",
                diff
            ).into());
        }

        Ok(())
    }
}
