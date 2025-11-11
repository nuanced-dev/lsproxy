use std::path::Path;

use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use lsproxy_common::utils::workspace_documents::{
    DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
};
use async_trait::async_trait;
use lsp_types::InitializeParams;
use std::error::Error;

pub struct GenericLspClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
    initialization_options: Option<serde_json::Value>,
    setup_workspace_method: Option<String>,
}

#[async_trait]
impl LspClient for GenericLspClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![] // Generic client doesn't specify root files
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
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
        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(workspace_folders),
            root_uri: Some(lsp_types::Url::from_file_path(&root_path).unwrap()),
            initialization_options: self.initialization_options.clone(),
            ..Default::default()
        })
    }

    async fn setup_workspace(
        &mut self,
        _root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(method) = self.setup_workspace_method.clone() {
            log::info!("Calling setup workspace method: {}", method);
            self.send_request(&method, None).await?;
        }
        Ok(())
    }
}

impl GenericLspClient {
    /// Create a new GenericLspClient with configurable file patterns and did-open behavior
    pub fn new(
        process: ProcessHandler,
        root_path: String,
        file_patterns: Vec<String>,
        did_open_config: DidOpenConfiguration,
    ) -> Self {
        let (_tx, rx) = tokio::sync::broadcast::channel(1);

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(&root_path),
            file_patterns,
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            rx,
            did_open_config,
        );

        let json_rpc_handler = JsonRpcHandler::new();

        Self {
            process,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
            initialization_options: None,
            setup_workspace_method: None,
        }
    }

    /// Set initialization options for the LSP server (e.g., Rust cargo.sysroot)
    pub fn with_initialization_options(mut self, options: serde_json::Value) -> Self {
        self.initialization_options = Some(options);
        self
    }

    /// Set setup workspace method to call after initialization (e.g., rust-analyzer/reloadWorkspace)
    pub fn with_setup_workspace_method(mut self, method: String) -> Self {
        self.setup_workspace_method = Some(method);
        self
    }

    /// Extract all components at once (consumes self)
    /// Returns (ProcessHandler, JsonRpcHandler, WorkspaceDocumentsHandler, PendingRequests)
    pub fn into_components(
        self,
    ) -> (
        ProcessHandler,
        JsonRpcHandler,
        WorkspaceDocumentsHandler,
        PendingRequests,
    ) {
        (
            self.process,
            self.json_rpc,
            self.workspace_documents,
            self.pending_requests,
        )
    }
}
