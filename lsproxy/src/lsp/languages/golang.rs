use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
        GOLANG_FILE_PATTERNS, GOLANG_ROOT_FILES,
    },
};
use async_trait::async_trait;
use log::{error, info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use notify_debouncer_mini::DebouncedEvent;
use std::{
    error::Error,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{process::Command, sync::broadcast::Receiver};

pub struct GoplsClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
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
        GOLANG_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
    }
    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }
    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;

        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            // Prefer workspaceFolders; do not also set root_uri.
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
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("gopls")
            .arg("-mode=stdio")
            .arg("-vv")
            .arg("-logfile=/tmp/gopls.log")
            .arg("-rpc.trace")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to start gopls process: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;

        let json_rpc_handler = JsonRpcHandler::new();

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            GOLANG_FILE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
            DidOpenConfiguration::Lazy,
        );

        let pending_requests = PendingRequests::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests,
        })
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
