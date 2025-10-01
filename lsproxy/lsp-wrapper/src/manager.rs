/// Simplified Manager for lsp-wrapper
///
/// Unlike the main LSProxy Manager that orchestrates multiple language servers,
/// this Manager wraps a single LSP client for the configured language.
use crate::api_types::{get_mount_dir, Identifier, Symbol};
use crate::ast_grep::client::AstGrepClient;
use crate::ast_grep::types::AstGrepMatch;
use crate::lsp::client::LspClient;
use crate::utils::file_utils::{absolute_path_to_relative_path_string, uri_to_relative_path_string};
use crate::utils::workspace_documents::WorkspaceDocuments;
use ignore::WalkBuilder;
use log::{error, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position, Range};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum LspManagerError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("No LSP client available for language")]
    NoLspClientAvailable,

    #[error("LSP client not found for {0}")]
    LspClientNotFound(crate::api_types::SupportedLanguages),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub struct Manager {
    // Box<dyn LspClient> for polymorphism - supports any language client
    // Mutex for interior mutability (LSP client needs &mut self)
    // Arc for shared ownership across actix-web handlers
    client: Arc<Mutex<Box<dyn LspClient>>>,
    ast_grep: AstGrepClient,
    workspace_path: String,
}

impl Manager {
    pub fn new(client: Arc<Mutex<Box<dyn LspClient>>>, workspace_path: String) -> Self {
        Self {
            client,
            ast_grep: AstGrepClient::new(),
            workspace_path,
        }
    }

    pub async fn list_files(&self) -> Result<Vec<String>, LspManagerError> {
        let mut files = Vec::new();

        for result in WalkBuilder::new(&self.workspace_path)
            .hidden(true)
            .parents(true)
            .git_ignore(true)
            .build()
        {
            match result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if let Ok(relative) = entry.path().strip_prefix(&self.workspace_path) {
                            if let Some(rel_str) = relative.to_str() {
                                files.push(rel_str.to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error walking workspace: {}", e);
                }
            }
        }

        Ok(files)
    }

    pub async fn get_file_identifiers(
        &self,
        file_path: &str,
    ) -> Result<Vec<Identifier>, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_identifiers(full_path_str)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;

        Ok(ast_grep_result.into_iter().map(|s| s.into()).collect())
    }

    pub async fn get_definitions_in_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<AstGrepMatch>, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_definitions_in_file(full_path_str)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;

        Ok(ast_grep_result)
    }

    pub async fn get_symbol_from_position(
        &self,
        file_path: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        match self
            .ast_grep
            .get_symbol_match_from_position(full_path_str, identifier_position)
            .await
        {
            Ok(ast_grep_symbol) => Ok(Symbol::from(ast_grep_symbol)),
            Err(e) => Err(LspManagerError::InternalError(e.to_string())),
        }
    }

    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Call LSP textDocument/definition directly (like base implementation)
        let mut locked_client = self.client.lock().await;
        let mut definition = locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
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
    ) -> Result<Vec<Location>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Call LSP textDocument/references
        let mut locked_client = self.client.lock().await;
        let mut locations = locked_client
            .text_document_reference(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("References retrieval failed: {}", e))
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
    ) -> Result<Vec<(AstGrepMatch, GotoDefinitionResponse)>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Get the symbol and its references using ast-grep
        let (_, references_to_symbols) = match self
            .ast_grep
            .get_symbol_and_references(full_path_str, &position, full_scan)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Err(LspManagerError::InternalError(format!(
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
            return Err(LspManagerError::InternalError(
                "Failed to retrieve any definitions for the referenced symbols".to_string(),
            ));
        }

        Ok(definitions)
    }

    pub async fn read_source_code(
        &self,
        file_path: &str,
        range: Option<Range>,
    ) -> Result<String, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let mut locked_client = self.client.lock().await;
        locked_client
            .get_workspace_documents()
            .read_text_document(&full_path, range)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Source code retrieval failed: {}", e))
            })
    }

    /// For health check - in lsp-wrapper, we always have a client
    pub fn get_client(&self, _lang: crate::api_types::SupportedLanguages) -> Option<()> {
        Some(())
    }
}
