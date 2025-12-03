use std::path::{Path, PathBuf};

use crate::lsp::client::CLIENT_CAPABILITES;
use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use async_trait::async_trait;
use common::api_types::JsonRpcMessage;
use log::{info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;

pub struct GoplsClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: common::utils::workspace_documents::WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
    unexpected_notifications_tx: tokio::sync::broadcast::Sender<JsonRpcMessage>,
}

#[async_trait]
impl LspClient for GoplsClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![] // Gopls doesn't use root files in the new architecture
    }

    fn get_workspace_documents(
        &mut self,
    ) -> &mut common::utils::workspace_documents::WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    fn get_unexpected_notifications_tx(&self) -> tokio::sync::broadcast::Sender<JsonRpcMessage> {
        self.unexpected_notifications_tx.clone()
    }

    #[allow(deprecated)]

    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;

        Ok(InitializeParams {
            capabilities: CLIENT_CAPABILITES.clone(),
            // Prefer workspaceFolders; do not also set root_uri to avoid confusion
            workspace_folders: Some(workspace_folders),
            root_uri: None,
            ..Default::default()
        })
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        let root = PathBuf::from(&root_path);

        // 1) Nearest ancestor with go.work
        if let Some(work_root) = nearest_ancestor_with(&root, "go.work") {
            info!(
                "Using nearest go.work at {:?} as single workspace root",
                work_root
            );
            let uri = Url::from_file_path(&work_root)
                .map_err(|_| format!("Failed to create URL from path: {}", work_root.display()))?;
            return Ok(vec![WorkspaceFolder {
                uri,
                name: work_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string(),
            }]);
        }

        // 2) Nearest ancestor with go.mod
        if let Some(mod_root) = nearest_ancestor_with(&root, "go.mod") {
            info!(
                "No go.work found; using nearest go.mod at {:?} as single workspace root",
                mod_root
            );
            let uri = Url::from_file_path(&mod_root)
                .map_err(|_| format!("Failed to create URL from path: {}", mod_root.display()))?;
            return Ok(vec![WorkspaceFolder {
                uri,
                name: mod_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string(),
            }]);
        }

        // 3) Fallback: use the provided root_path
        warn!(
            "No go.work or go.mod found in ancestors. Falling back to provided root: {}",
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

impl GoplsClient {
    /// Create a new GoplsClient from the existing GenericLspClient components
    pub fn new(
        process: ProcessHandler,
        json_rpc: JsonRpcHandler,
        workspace_documents: common::utils::workspace_documents::WorkspaceDocumentsHandler,
        pending_requests: PendingRequests,
    ) -> Self {
        let (unexpected_notifications_tx, _) = tokio::sync::broadcast::channel(1);
        Self {
            process,
            json_rpc,
            workspace_documents,
            pending_requests,
            unexpected_notifications_tx,
        }
    }
}

/// Walk upward from `start` to filesystem root, returning the first directory
/// that contains a child named `needle` (e.g., "go.work" or "go.mod").
fn nearest_ancestor_with(start: &Path, needle: &str) -> Option<PathBuf> {
    let mut cur = start;

    // If `start` is a file path, prefer its parent.
    if cur.is_file() {
        cur = cur.parent()?;
    }

    let mut dir = cur.to_path_buf();
    loop {
        let candidate = dir.join(needle);
        if candidate.exists() {
            return Some(dir);
        }

        // Stop at filesystem root
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}
