use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::{
        file_utils::{search_paths, FileType},
        workspace_documents::{
            DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
            GOLANG_FILE_PATTERNS, GOLANG_ROOT_FILES,
        },
    },
};
use async_trait::async_trait;
use log::{error, info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use notify_debouncer_mini::DebouncedEvent;
use std::{error::Error, path::Path, process::Stdio};
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
            workspace_folders: Some(workspace_folders.clone()),
            root_uri: workspace_folders.first().map(|f| f.uri.clone()), // <--------- Not default behavior
            ..Default::default()
        })
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        // Step 1: Check for go.work file at the root path
        let go_work_path = Path::new(&root_path).join("go.work");

        if go_work_path.exists() {
            info!(
                "Found go.work file at {:?}, using single workspace mode",
                go_work_path
            );
            // Use ONLY the go.work workspace - this prevents gopls from
            // creating multiple build scopes for each module
            let uri = Url::from_file_path(&root_path)
                .map_err(|_| format!("Failed to create URL from root path: {}", root_path))?;

            return Ok(vec![WorkspaceFolder {
                uri,
                name: Path::new(&root_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string(),
            }]);
        }

        // Step 2: No go.work found - fall back to finding individual go.mod files
        // and using their parent directories as workspace folders.
        info!("No go.work file found, searching for go.mod files");
        let mut workspace_folders: Vec<WorkspaceFolder> = Vec::new();
        let include_patterns = vec!["**/go.mod".to_string()];
        let exclude_patterns = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        match search_paths(
            Path::new(&root_path),
            include_patterns,
            exclude_patterns,
            true,
            FileType::File,  // Search for go.mod FILES, not directories
        ) {
            Ok(go_mod_files) => {
                // For each go.mod file, use its parent directory as a workspace folder
                for go_mod_file in go_mod_files {
                    if let Some(module_dir) = go_mod_file.parent() {
                        if let Ok(uri) = Url::from_file_path(module_dir) {
                            workspace_folders.push(WorkspaceFolder {
                                uri,
                                name: module_dir
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }

        // Step 3: Fallback if nothing found - use root path itself
        if workspace_folders.is_empty() {
            warn!("No go.mod directories found. Using root path as workspace.");
            if let Ok(uri) = Url::from_file_path(&root_path) {
                workspace_folders.push(WorkspaceFolder {
                    uri,
                    name: Path::new(&root_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("workspace")
                        .to_string(),
                });
            }
        }

        info!(
            "Found {} workspace folder(s) for gopls",
            workspace_folders.len()
        );
        Ok(workspace_folders)
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
