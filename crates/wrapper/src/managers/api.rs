use crate::lsp::client::LspClient;
/// Simplified Manager for lsp-wrapper
///
/// Unlike the main Nuanced LSP Manager that orchestrates multiple language servers,
/// this Manager wraps a single LSP client for the configured language.
use common::api_types::{get_mount_dir, Identifier, JsonRpcMessage, Symbol};
use common::ast_grep::client::AstGrepClient;
use common::ast_grep::types::AstGrepMatch;
use common::utils::file_utils::uri_to_relative_path_string;
use common::utils::workspace_documents::WorkspaceDocuments;
use log::{error, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position, Range};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum ApiManagerError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("No LSP client available for language")]
    NoLspClientAvailable,

    #[error("LSP client not found for {0}")]
    LspClientNotFound(common::api_types::SupportedLanguages),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Threadsafe manager that maps API calls to invocations to the local LSP server.
pub struct ApiManager {
    // Box<dyn LspClient> for polymorphism - supports any language client
    // Mutex for interior mutability (LSP client needs &mut self)
    // Arc for shared ownership across actix-web handlers
    client: Arc<Mutex<Box<dyn LspClient>>>,
    ast_grep: AstGrepClient,
}

impl ApiManager {
    pub fn new(client: Arc<Mutex<Box<dyn LspClient>>>) -> Self {
        Self {
            client,
            ast_grep: AstGrepClient::new_wrapper(),
        }
    }

    pub async fn get_file_identifiers(
        &self,
        file_path: &str,
    ) -> Result<Vec<Identifier>, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);

        if !full_path.exists() {
            return Err(ApiManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_identifiers(full_path_str)
            .await
            .map_err(|e| {
                ApiManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;

        Ok(ast_grep_result.into_iter().map(|s| s.into()).collect())
    }

    pub async fn get_definitions_in_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<AstGrepMatch>, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);

        if !full_path.exists() {
            return Err(ApiManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_definitions_in_file(full_path_str)
            .await
            .map_err(|e| {
                ApiManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;

        Ok(ast_grep_result)
    }

    pub async fn get_symbol_from_position(
        &self,
        file_path: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        match self
            .ast_grep
            .get_symbol_match_from_position(full_path_str, identifier_position)
            .await
        {
            Ok(ast_grep_symbol) => Ok(Symbol::from(ast_grep_symbol)),
            Err(e) => Err(ApiManagerError::InternalError(e.to_string())),
        }
    }

    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);

        if !full_path.exists() {
            return Err(ApiManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Call LSP textDocument/definition directly (like base implementation)
        let mut locked_client = self.client.lock().await;
        let mut definition = locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                ApiManagerError::InternalError(format!("Definition retrieval failed: {}", e))
            })?;

        // Sort the locations if there are multiple
        match &mut definition {
            GotoDefinitionResponse::Array(locations) => {
                locations.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.uri);
                    let path_b = uri_to_relative_path_string(&b.uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.range.start.line.cmp(&b.range.start.line))
                        .then(a.range.start.character.cmp(&b.range.start.character))
                });
            }
            GotoDefinitionResponse::Link(links) => {
                links.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.target_uri);
                    let path_b = uri_to_relative_path_string(&b.target_uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.target_range.start.line.cmp(&b.target_range.start.line))
                        .then(
                            a.target_range
                                .start
                                .character
                                .cmp(&b.target_range.start.character),
                        )
                });
            }
            _ => {}
        }

        Ok(definition)
    }

    pub async fn find_references(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);

        if !full_path.exists() {
            return Err(ApiManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Call LSP textDocument/references
        let mut locked_client = self.client.lock().await;
        let mut locations = locked_client
            .text_document_reference(full_path_str, position)
            .await
            .map_err(|e| {
                ApiManagerError::InternalError(format!("References retrieval failed: {}", e))
            })?;

        // Sort locations
        locations.sort_by(|a, b| {
            let path_a = uri_to_relative_path_string(&a.uri);
            let path_b = uri_to_relative_path_string(&b.uri);
            path_a
                .cmp(&path_b)
                .then(a.range.start.line.cmp(&b.range.start.line))
                .then(a.range.start.character.cmp(&b.range.start.character))
        });

        Ok(locations)
    }

    pub async fn find_referenced_symbols(
        &self,
        file_path: &str,
        position: Position,
        full_scan: bool,
    ) -> Result<Vec<(AstGrepMatch, GotoDefinitionResponse)>, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);

        if !full_path.exists() {
            return Err(ApiManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Get the symbol and its references using ast-grep
        let (_, references_to_symbols) = match self
            .ast_grep
            .get_symbol_and_references(full_path_str, &position, full_scan)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiManagerError::InternalError(format!(
                    "Failed to find referenced symbols, {}",
                    e
                )));
            }
        };

        let mut definitions = Vec::new();
        let mut locked_client = self.client.lock().await;

        // Get LSP definitions for each reference
        for ast_match in references_to_symbols.iter() {
            let lsp_position = lsp_types::Position::from(ast_match);

            match locked_client
                .text_document_definition(full_path_str, lsp_position)
                .await
            {
                Ok(definition) => {
                    definitions.push((ast_match.clone(), definition));
                }
                Err(e) => {
                    warn!(
                        "Definition retrieval failed for reference: {}, error: {}",
                        ast_match.meta_variables.single.name.text, e
                    );
                }
            }
        }

        // Only return an error if we couldn't get any definitions at all
        if definitions.is_empty() && !references_to_symbols.is_empty() {
            return Err(ApiManagerError::InternalError(
                "Failed to retrieve any definitions for the referenced symbols".to_string(),
            ));
        }

        Ok(definitions)
    }

    pub async fn read_source_code(
        &self,
        file_path: &str,
        range: Option<Range>,
    ) -> Result<String, ApiManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let mut locked_client = self.client.lock().await;
        locked_client
            .get_workspace_documents()
            .read_text_document(&full_path, range)
            .await
            .map_err(|e| {
                ApiManagerError::InternalError(format!("Source code retrieval failed: {}", e))
            })
    }

    /// For health check - in lsp-wrapper, we always have a client
    pub fn get_client(&self, _lang: common::api_types::SupportedLanguages) -> Option<()> {
        Some(())
    }

    /// Forward a raw LSP JSON-RPC request to the LSP server
    ///
    /// This provides lightweight pass-through of JSON-RPC requests with minimal processing.
    pub async fn lsp(&self, request: JsonRpcMessage) -> Result<JsonRpcMessage, ApiManagerError> {
        let req_id = request.id;
        let Some(method) = request.method else {
            return Err(ApiManagerError::BadRequest("missing method".to_string()));
        };

        // Forward to LSP server
        let mut locked_client = self.client.lock().await;
        let result = locked_client
            .send_request(&method, request.params)
            .await
            .map_err(|e| {
                error!("Failed to forward LSP request: {}", e);
                ApiManagerError::InternalError(format!("LSP request failed: {}", e))
            })?;

        // Build response
        let response = JsonRpcMessage::new_result_response(req_id, result);

        Ok(response)
    }
}

// Convert from common LspError to wrapper-specific LspManagerError
impl From<common::error::LspError> for ApiManagerError {
    fn from(err: common::error::LspError) -> Self {
        use common::error::LspError as CommonError;
        match &err {
            CommonError::FileNotFound(s) => ApiManagerError::FileNotFound(s.clone()),
            CommonError::UnsupportedFileType(s) => ApiManagerError::UnsupportedFileType(s.clone()),
            _ => ApiManagerError::InternalError(err.to_string()),
        }
    }
}
