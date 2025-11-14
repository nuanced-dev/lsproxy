use std::path::PathBuf;
use std::fs;

use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use async_trait::async_trait;
use log::{info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;

pub struct SorbetClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: lsproxy_common::utils::workspace_documents::WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for SorbetClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![] // Sorbet doesn't use root files in the new architecture
    }

    fn get_workspace_documents(
        &mut self,
    ) -> &mut lsproxy_common::utils::workspace_documents::WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    #[allow(deprecated)]


    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;

        // Sorbet initialization options for lazy indexing
        let init_options = serde_json::json!({
            "sorbet.lsp.lazyIndexing": true
        });

        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(workspace_folders),
            root_uri: None,
            initialization_options: Some(init_options),
            ..Default::default()
        })
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        info!("SorbetClient::find_workspace_folders called with root_path: {}", root_path);
        let root = PathBuf::from(&root_path);

        // 1) Look for sorbet/config file
        let sorbet_config_path = root.join("sorbet").join("config");
        info!("Looking for sorbet/config at {:?}, exists: {}", sorbet_config_path, sorbet_config_path.exists());
        if sorbet_config_path.exists() {
            info!("Found sorbet/config at {:?}", sorbet_config_path);

            // Parse the config file to find --dir entries
            match fs::read_to_string(&sorbet_config_path) {
                Ok(contents) => {
                    let mut workspace_folders = Vec::new();
                    let mut lines = contents.lines();

                    while let Some(line) = lines.next() {
                        let trimmed = line.trim();

                        // Look for --dir option
                        if trimmed == "--dir" {
                            // The directory path should be on the next line
                            if let Some(dir_line) = lines.next() {
                                let dir_path = dir_line.trim();
                                let full_path = root.join(dir_path);

                                if full_path.exists() {
                                    info!("Adding Sorbet workspace folder: {:?}", full_path);

                                    let uri = Url::from_file_path(&full_path)
                                        .map_err(|_| format!("Failed to create URL from path: {}", full_path.display()))?;

                                    workspace_folders.push(WorkspaceFolder {
                                        uri,
                                        name: full_path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(dir_path)
                                            .to_string(),
                                    });
                                } else {
                                    warn!("Sorbet config specifies directory that doesn't exist: {}", dir_path);
                                }
                            }
                        }
                    }

                    if !workspace_folders.is_empty() {
                        return Ok(workspace_folders);
                    }

                    warn!("No valid --dir entries found in sorbet/config");
                }
                Err(e) => {
                    warn!("Failed to read sorbet/config: {}", e);
                }
            }
        }

        // 2) Fallback: use the provided root_path
        warn!(
            "No sorbet/config found or no valid directories specified. Falling back to root: {}",
            root.display()
        );
        let uri = Url::from_file_path(&root)
            .map_err(|_| format!("Failed to create URL from root path: {}", root.display()))?;
        Ok(vec![WorkspaceFolder {
            uri,
            name: root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("workspace")
                .to_string(),
        }])
    }
}

impl SorbetClient {
    /// Create a new SorbetClient from the existing GenericLspClient components
    pub fn new(
        process: ProcessHandler,
        json_rpc: JsonRpcHandler,
        workspace_documents: lsproxy_common::utils::workspace_documents::WorkspaceDocumentsHandler,
        pending_requests: PendingRequests,
    ) -> Self {
        Self {
            process,
            json_rpc,
            workspace_documents,
            pending_requests,
        }
    }
}
