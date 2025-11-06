use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
        PHP_FILE_PATTERNS, PHP_ROOT_FILES,
    },
};
use async_trait::async_trait;
use log::warn;
use lsp_types::InitializeParams;
use notify_debouncer_mini::DebouncedEvent;
use std::{error::Error, path::Path, process::Stdio};
use tokio::{process::Command, sync::broadcast::Receiver};
use url::Url;

pub struct PhpactorClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for PhpactorClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }
    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }
    fn get_root_files(&mut self) -> Vec<String> {
        PHP_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
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
        let workspace_folders = self
            .find_workspace_folders(root_path.clone())
            .await
            .unwrap();
        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(workspace_folders.clone()),
            root_uri: Some(Url::from_file_path(&root_path).map_err(|_| "Invalid root path")?),
            ..Default::default()
        })
    }
}

impl PhpactorClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create a Phpactor configuration file
        let config_path = Path::new(root_path).join(".phpactor.json");
        let config_content = serde_json::json!({
            "logging.enabled": true,
            "logging.level": "info",
            "logging.path": "/tmp/phpactor.log",
            "logging.formatter": "json",
            "language_server.trace": false,
        });

        std::fs::write(&config_path, serde_json::to_string_pretty(&config_content)?)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Dump autoload if it exists for better performance
        let mut child = Command::new("composer")
            .arg("dump-autoload")
            .arg("--no-scripts")
            .current_dir(root_path) // Set the working directory
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()) // Capture stderr
            .spawn()
            .map_err(|e| format!("Failed to spawn `composer dump-autoload`: {}", e))?;

        // Wait for the child process to complete
        if let Ok(status) = child.wait().await {
            if !status.success() {
                if let Some(code) = status.code() {
                    warn!(
                        "`composer dump-autoload` exited with non-zero status code: {}",
                        code
                    );
                } else {
                    warn!("`composer dump-autoload` was terminated by a signal.");
                }
            }
        }

        let process = Command::new("phpactor")
            .arg("language-server")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            PHP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
            DidOpenConfiguration::Lazy,
        );

        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
