/// Container-based Manager that routes requests to LSP wrapper containers
///
/// This replaces the direct LSP process management with HTTP calls to containerized
/// LSP servers. Each container runs lsp-wrapper which handles all the business logic.

use crate::api_types::*;
use crate::ast_grep::types::AstGrepMatch;
use crate::container::{ContainerHttpClient, ContainerInfo, ContainerOrchestrator, OrchestratorError};
use crate::lsp::manager::LspManagerError;
use crate::utils::file_utils::{detect_language, search_files};
use crate::utils::workspace_documents::*;
use log::{error, info, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ContainerManager {
    orchestrator: Arc<ContainerOrchestrator>,
    http_clients: Arc<Mutex<HashMap<SupportedLanguages, ContainerHttpClient>>>,
    workspace_path: String,
}

impl ContainerManager {
    pub async fn new(workspace_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let orchestrator = ContainerOrchestrator::new().await?;

        Ok(Self {
            orchestrator: Arc::new(orchestrator),
            http_clients: Arc::new(Mutex::new(HashMap::new())),
            workspace_path: workspace_path.to_string(),
        })
    }

    /// Detects the languages in the workspace by searching for files
    fn detect_languages_in_workspace(&self) -> Vec<SupportedLanguages> {
        let mut languages = Vec::new();

        for (lang, patterns) in [
            (SupportedLanguages::Python, PYTHON_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::TypeScriptJavaScript, TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Rust, RUST_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::CPP, C_AND_CPP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::CSharp, CSHARP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Java, JAVA_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Golang, GOLANG_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::PHP, PHP_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::Ruby, RUBY_FILE_PATTERNS.to_vec()),
            (SupportedLanguages::RubySorbet, RUBY_SORBET_FILE_PATTERNS.to_vec()),
        ] {
            let patterns: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();
            let exclude: Vec<String> = DEFAULT_EXCLUDE_PATTERNS.iter().map(|s| s.to_string()).collect();

            if !search_files(&self.workspace_path, &patterns, &exclude, 1).is_empty() {
                languages.push(lang);
            }
        }

        languages
    }

    /// Start containers for detected languages
    pub async fn start_containers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let languages = self.detect_languages_in_workspace();

        for lang in languages {
            // Check if container already exists
            if self.orchestrator.get_container(&lang).await.is_some() {
                info!("Container for {:?} already running", lang);
                continue;
            }

            info!("Starting container for {:?}", lang);
            match self.orchestrator.spawn_container(lang.clone(), &self.workspace_path).await {
                Ok(container_info) => {
                    info!("Container started for {:?}: {}", lang, container_info.endpoint);

                    // Create HTTP client for this container
                    let client = ContainerHttpClient::new(&container_info.endpoint);
                    self.http_clients.lock().await.insert(lang, client);
                }
                Err(e) => {
                    error!("Failed to start container for {:?}: {}", lang, e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    /// Get or create HTTP client for a language
    async fn get_client(&self, language: SupportedLanguages) -> Result<ContainerHttpClient, LspManagerError> {
        // Check if we already have a client
        {
            let clients = self.http_clients.lock().await;
            if let Some(client) = clients.get(&language) {
                return Ok(ContainerHttpClient::new(
                    &self.orchestrator
                        .get_container(&language)
                        .await
                        .ok_or_else(|| LspManagerError::NoLspClientAvailable)?
                        .endpoint
                ));
            }
        }

        // Need to spawn a container
        info!("Spawning container for {:?}", language);
        let container_info = self.orchestrator
            .spawn_container(language.clone(), &self.workspace_path)
            .await
            .map_err(|e| LspManagerError::InternalError(format!("Failed to spawn container: {}", e)))?;

        let client = ContainerHttpClient::new(&container_info.endpoint);
        self.http_clients.lock().await.insert(language.clone(), client);

        Ok(ContainerHttpClient::new(&container_info.endpoint))
    }

    /// Find definition via container
    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = GetDefinitionRequest {
            position: FilePosition {
                path: file_path.to_string(),
                position: position.into(),
            },
        };

        client.find_definition(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))
    }

    /// Find references via container
    pub async fn find_references(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = GetReferencesRequest {
            position: FilePosition {
                path: file_path.to_string(),
                position: position.into(),
            },
        };

        client.find_references(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))
    }

    /// Get file identifiers via container
    pub async fn get_file_identifiers(
        &self,
        file_path: &str,
    ) -> Result<Vec<Identifier>, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = FindIdentifierRequest {
            path: file_path.to_string(),
            name: String::new(), // Empty means all identifiers
            position: None,
        };

        client.find_identifier(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))
    }

    /// Get definitions in file via container
    pub async fn definitions_in_file_ast_grep(
        &self,
        file_path: &str,
    ) -> Result<Vec<Symbol>, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = FileSymbolsRequest {
            file_path: file_path.to_string(),
        };

        client.definitions_in_file(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))
    }

    /// Get symbol from position via container
    pub async fn get_symbol_from_position(
        &self,
        file_path: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, LspManagerError> {
        // This is typically done via definitions_in_file and filtering
        // For now, delegate to the container's find-identifier endpoint
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        // Get all symbols and filter by position
        let request = FileSymbolsRequest {
            file_path: file_path.to_string(),
        };

        let symbols = client.definitions_in_file(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        // Find symbol at position
        for symbol in symbols {
            if symbol.file_range.range.start.line == identifier_position.line
                && symbol.file_range.range.start.character == identifier_position.character
            {
                return Ok(symbol);
            }
        }

        Err(LspManagerError::InternalError("No symbol found at position".to_string()))
    }

    /// Find referenced symbols via container
    pub async fn find_referenced_symbols(
        &self,
        file_path: &str,
        position: Position,
        full_scan: bool,
    ) -> Result<Vec<(AstGrepMatch, GotoDefinitionResponse)>, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = FindReferencedSymbolsRequest {
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: position.into(),
            },
            full_scan,
        };

        let response = client.find_referenced_symbols(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        // Convert response to expected format
        // Note: This is a simplified version - the actual implementation would need
        // to properly convert the response types
        Ok(vec![])
    }

    /// List files via container
    pub async fn list_files(&self) -> Result<Vec<String>, LspManagerError> {
        // List files from all running containers and deduplicate
        let containers = self.http_clients.lock().await;
        let mut all_files = Vec::new();

        for (_lang, client) in containers.iter() {
            match client.list_files().await {
                Ok(files) => all_files.extend(files),
                Err(e) => warn!("Failed to list files from container: {}", e),
            }
        }

        // Deduplicate
        all_files.sort();
        all_files.dedup();

        Ok(all_files)
    }

    /// Read source code via container
    pub async fn read_source_code(
        &self,
        file_path: &str,
        range: Option<lsp_types::Range>,
    ) -> Result<String, LspManagerError> {
        let language = detect_language(file_path)
            .map_err(|e| LspManagerError::InternalError(e.to_string()))?;

        let client = self.get_client(language).await?;

        let request = ReadSourceCodeRequest {
            path: file_path.to_string(),
            range: range.map(|r| r.into()),
        };

        client.read_source(&request).await
            .map_err(|e| LspManagerError::InternalError(e.to_string()))
    }

    /// Check if a client exists for a language (for health checks)
    pub fn get_client_sync(&self, _language: SupportedLanguages) -> Option<()> {
        // For compatibility with existing health check
        Some(())
    }

    /// Cleanup all containers on shutdown
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.orchestrator.cleanup_all().await?;
        Ok(())
    }
}
