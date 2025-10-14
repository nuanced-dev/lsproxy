use std::path::Path;

use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use crate::utils::workspace_documents::{
    DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
};
use async_trait::async_trait;

pub struct GenericLspClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
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
        }
    }
}
