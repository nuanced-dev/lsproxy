use async_trait::async_trait;
use common::utils::file_utils::fix_relative_uris;
use common::utils::language_utils::detect_language_string;
use log::{debug, error, info, warn};
use lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    GeneralClientCapabilities, GotoDefinitionParams, GotoDefinitionResponse, InitializeParams,
    InitializeResult, Location, PartialResultParams, Position, PositionEncodingKind,
    PublishDiagnosticsClientCapabilities, ReferenceContext, ReferenceParams, TagSupport,
    TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, WorkDoneProgressParams,
};
use std::error::Error;
use std::path::PathBuf;
use std::sync::LazyLock;

use common::utils::workspace_documents::{
    DidOpenConfiguration, WorkspaceDocuments, WorkspaceDocumentsHandler,
};

use crate::lsp::json_rpc::JsonRpc;
use crate::lsp::process::Process;
use crate::lsp::{ExpectedMessageKey, JsonRpcHandler, PendingRequests, ProcessHandler};

#[async_trait]
pub trait LspConfig: Send + Sync {
    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>>;

    fn get_setup_workspace_method(&self) -> Option<String> {
        None
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![".git".to_string()]
    }

    fn include_patterns(&self) -> Vec<String>;

    fn exclude_patterns(&self) -> Vec<String>;

    fn did_open_configuration(&self) -> DidOpenConfiguration;
}

pub(crate) static CLIENT_CAPABILITES: LazyLock<ClientCapabilities> = LazyLock::new(|| {
    let mut capabilities = ClientCapabilities::default();
    capabilities.general = Some(GeneralClientCapabilities {
        position_encodings: Some(vec![PositionEncodingKind::UTF16]),
        ..Default::default()
    });
    capabilities.text_document = Some(TextDocumentClientCapabilities {
        document_symbol: Some(DocumentSymbolClientCapabilities {
            dynamic_registration: Some(false),
            hierarchical_document_symbol_support: Some(true),
            ..Default::default()
        }),
        // Turn off diagnostics for performance, we don't use them at the moment
        publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
            related_information: Some(false),
            tag_support: Some(TagSupport { value_set: vec![] }),
            code_description_support: Some(false),
            data_support: Some(false),
            version_support: Some(false),
        }),
        ..Default::default()
    });

    capabilities.experimental = Some(serde_json::json!({
        "serverStatusNotification": true
    }));
    capabilities
});

pub struct LspClient {
    config: Box<dyn LspConfig>,
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
    unexpected_notifications_tx: tokio::sync::broadcast::Sender<common::api_types::JsonRpcMessage>,
}

impl LspClient {
    pub fn new(config: Box<dyn LspConfig>, process: ProcessHandler, root_path: &str) -> Self {
        let json_rpc = JsonRpcHandler::new();
        let pending_requests = PendingRequests::new();
        let (unexpected_notifications_tx, _) =
            tokio::sync::broadcast::channel::<common::api_types::JsonRpcMessage>(1);
        let (_, workspace_docs_rx) =
            tokio::sync::broadcast::channel::<notify_debouncer_mini::DebouncedEvent>(1);

        let include_patterns = config.include_patterns();
        let exclude_patterns = config.exclude_patterns();
        let did_open_configuration = config.did_open_configuration();

        let workspace_documents = WorkspaceDocumentsHandler::new(
            std::path::Path::new(root_path),
            include_patterns,
            exclude_patterns,
            workspace_docs_rx,
            did_open_configuration,
        );

        Self {
            config,
            process,
            json_rpc,
            workspace_documents,
            pending_requests,
            unexpected_notifications_tx,
        }
    }

    pub async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        info!("Initializing LSP client with root path: {:?}", root_path);
        self.start_response_listener().await?;

        let params = self.config.get_initialize_params(root_path.clone()).await?;

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        debug!("Initialization successful: {:?}", init_result);
        self.send_initialized().await?;

        Ok(init_result)
    }

    pub async fn send_notification(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let notification = self.json_rpc.create_notification(method, params);
        debug!("Sending notification: {}", method);
        self.process.send(&notification).await?;
        Ok(())
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let (id, request) = self.json_rpc.create_request(method, params);

        let mut response_receiver = self.pending_requests.add_request(id).await?;

        debug!("Sending request {}: {}", id, method);
        self.process.send(&request).await?;

        let response = response_receiver
            .recv()
            .await
            .map_err(|e| format!("Failed to receive response: {}", e))?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error.clone() {
            error!("Recieved error: {:?}", response);
            if error.message.starts_with("KeyError") {
                return Ok(serde_json::Value::Array(vec![]));
            }
            Err(error.into())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    async fn start_response_listener(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut process = self.process.clone();
        let pending_requests = self.pending_requests.clone();
        let json_rpc = self.json_rpc.clone();
        let notification_channel = self.unexpected_notifications_tx.clone();

        tokio::spawn(async move {
            loop {
                if let Ok(raw_response) = process.receive().await {
                    if let Ok(message) = json_rpc.parse_message(&raw_response) {
                        if let Some(id) = &message.id {
                            // we always use u64 ids here, so the server process should respond with those as well
                            let Some(id) = id.as_u64() else {
                                debug!("Message has invalid id type: {:?}", message.id);
                                continue;
                            };
                            debug!("Received response for request {}", id);
                            if let Ok(Some(sender)) = pending_requests.remove_request(id).await {
                                if sender.send(message.clone()).is_err() {
                                    error!("Failed to send response for request {}", id);
                                }
                            } else {
                                let response = json_rpc.create_success_response(id);
                                let _ = process.send(&response).await;
                            }
                        } else if let Some(method) = message.method.clone() {
                            debug!("Received notification {}", method);
                            let mut handled = false;
                            if let Some(params) = &message.params {
                                let message_key = ExpectedMessageKey {
                                    method,
                                    params: params.clone(),
                                };
                                if let Some(sender) =
                                    pending_requests.remove_notification(message_key).await
                                {
                                    handled = true;
                                    sender.send(message.clone()).unwrap();
                                }
                            }
                            if !handled {
                                let _ = notification_channel.send(message);
                            }
                        } else {
                            debug!("Received unexpected message: {:?}", message);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn send_initialized(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Sending initialized");
        self.send_notification("initialized", Some(serde_json::json!({})))
            .await
    }

    async fn text_document_did_open(
        &mut self,
        item: lsp_types::TextDocumentItem,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let params = DidOpenTextDocumentParams {
            text_document: item,
        };
        self.send_notification("textDocument/didOpen", Some(serde_json::to_value(params)?))
            .await
    }

    pub async fn text_document_definition(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, Box<dyn Error + Send + Sync>> {
        debug!(
            "Requesting goto definition for {}, line {}, character {}",
            file_path, position.line, position.character
        );

        let needs_open = {
            let workspace_documents = &self.workspace_documents;
            workspace_documents.get_did_open_configuration() == DidOpenConfiguration::Lazy
                && !workspace_documents.is_did_open_document(file_path)
        };

        // If needed, read the document text and send didOpen
        if needs_open {
            info!("Sending textDocument/didOpen for {}", file_path);
            let document_text = self
                .workspace_documents
                .read_text_document(&PathBuf::from(file_path), None)
                .await?;

            self.text_document_did_open(TextDocumentItem {
                uri: Url::from_file_path(file_path).unwrap(),
                language_id: detect_language_string(file_path)?,
                version: 1,
                text: document_text,
            })
            .await?;

            self.workspace_documents.add_did_open_document(file_path);
        }

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap(),
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = self
            .send_request(
                "textDocument/definition",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        // If result is null, default to an empty array response instead of failing deserialization
        let goto_resp: GotoDefinitionResponse = if result.is_null() {
            GotoDefinitionResponse::Array(Vec::new())
        } else {
            // Pre-process the result to fix relative URIs (Sorbet issue)
            let workspace_path = self
                .workspace_documents
                .root_path()
                .to_str()
                .ok_or("Invalid workspace path")?;
            let preprocessed_result = fix_relative_uris(result, workspace_path);

            // Try standard deserialization first
            match serde_json::from_value(preprocessed_result.clone()) {
                Ok(resp) => resp,
                Err(e) => {
                    // If standard deserialization fails, try parsing as array or single location
                    // This handles non-standard LSP implementations like Sorbet
                    warn!("Standard deserialization failed: {}. Attempting fallback parsing. Raw response: {}", e, serde_json::to_string(&preprocessed_result).unwrap_or_else(|_| "Unable to serialize".to_string()));

                    // Try as array of locations
                    if let Ok(locs) =
                        serde_json::from_value::<Vec<Location>>(preprocessed_result.clone())
                    {
                        GotoDefinitionResponse::Array(locs)
                    }
                    // Try as single location
                    else if let Ok(loc) =
                        serde_json::from_value::<Location>(preprocessed_result.clone())
                    {
                        GotoDefinitionResponse::Scalar(loc)
                    }
                    // Give up but include the raw response in the error
                    else {
                        return Err(format!(
                            "Failed to parse goto definition response: {}. Raw response: {}",
                            e,
                            serde_json::to_string(&preprocessed_result)
                                .unwrap_or_else(|_| "Unable to serialize".to_string())
                        )
                        .into());
                    }
                }
            }
        };

        debug!("Received goto definition response");
        Ok(goto_resp)
    }

    pub async fn text_document_reference(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        debug!(
            "Requesting goto references for {}, line {}, character {}",
            file_path, position.line, position.character
        );

        // Get the configuration and check if document is opened first
        let needs_open = {
            let workspace_documents = &self.workspace_documents;
            workspace_documents.get_did_open_configuration() == DidOpenConfiguration::Lazy
                && !workspace_documents.is_did_open_document(file_path)
        };

        // If needed, read the document text and send didOpen
        if needs_open {
            info!("Sending textDocument/didOpen for {}", file_path);
            let document_text = self
                .workspace_documents
                .read_text_document(&PathBuf::from(file_path), None)
                .await?;

            self.text_document_did_open(TextDocumentItem {
                uri: Url::from_file_path(file_path).unwrap(),
                language_id: detect_language_string(file_path)?,
                version: 1,
                text: document_text,
            })
            .await?;

            self.workspace_documents.add_did_open_document(file_path);
        }

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self
            .send_request(
                "textDocument/references",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        let ref_resp: Vec<Location> = if result.is_null() {
            Vec::new()
        } else {
            // Pre-process the result to fix relative URIs (Sorbet issue)
            let workspace_path = self
                .workspace_documents
                .root_path()
                .to_str()
                .ok_or("Invalid workspace path")?;
            let preprocessed_result = fix_relative_uris(result, workspace_path);
            serde_json::from_value(preprocessed_result)?
        };
        debug!("Received references response");
        Ok(ref_resp)
    }

    pub fn subscribe_to_unexpected_notifications(
        &self,
    ) -> tokio::sync::broadcast::Receiver<common::api_types::JsonRpcMessage> {
        self.unexpected_notifications_tx.subscribe()
    }

    pub async fn setup_workspace(
        &mut self,
        _root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(method) = self.config.get_setup_workspace_method() {
            info!("Calling setup workspace method: {}", method);
            self.send_request(&method, None).await?;
        }
        Ok(())
    }

    pub async fn expect_notification(
        &mut self,
        key: ExpectedMessageKey,
    ) -> Result<
        tokio::sync::broadcast::Receiver<common::api_types::JsonRpcMessage>,
        Box<dyn Error + Send + Sync>,
    > {
        self.pending_requests.add_notification(key).await
    }

    pub fn get_workspace_documents(&self) -> &WorkspaceDocumentsHandler {
        &self.workspace_documents
    }
}
