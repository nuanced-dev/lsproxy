use crate::lsp::client::{LspConfig, CLIENT_CAPABILITES};

use async_trait::async_trait;
use common::utils::file_utils::{search_paths, FileType};
use common::utils::workspace_documents::{DidOpenConfiguration, DEFAULT_EXCLUDE_PATTERNS};
use log::warn;
use lsp_types::{InitializeParams, WorkspaceFolder};
use std::error::Error;
use std::path::Path;
use url::Url;

#[derive(Clone)]
pub struct GenericConfig {
    initialization_options: Option<serde_json::Value>,
    setup_workspace_method: Option<String>,
    file_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    did_open_configuration: DidOpenConfiguration,
}

#[async_trait]
impl LspConfig for GenericConfig {
    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        Ok(InitializeParams {
            capabilities: CLIENT_CAPABILITES.clone(),
            workspace_folders: Some(workspace_folders),
            #[allow(deprecated)]
            root_uri: Some(Url::from_file_path(&root_path).unwrap()),
            initialization_options: self.initialization_options.clone(),
            ..Default::default()
        })
    }

    fn get_setup_workspace_method(&self) -> Option<String> {
        self.setup_workspace_method.clone()
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![]
    }

    fn include_patterns(&self) -> Vec<String> {
        self.file_patterns.clone()
    }

    fn exclude_patterns(&self) -> Vec<String> {
        self.exclude_patterns.clone()
    }

    fn did_open_configuration(&self) -> DidOpenConfiguration {
        self.did_open_configuration.clone()
    }
}

impl GenericConfig {
    pub fn new(
        file_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        did_open_configuration: DidOpenConfiguration,
    ) -> Self {
        Self {
            initialization_options: None,
            setup_workspace_method: None,
            file_patterns,
            exclude_patterns,
            did_open_configuration,
        }
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        let mut workspace_folders: Vec<WorkspaceFolder> = Vec::new();
        let include_patterns = self
            .get_root_files()
            .into_iter()
            .map(|f| format!("**/{f}"))
            .collect();
        let exclude_patterns = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        match search_paths(
            Path::new(&root_path),
            include_patterns,
            exclude_patterns,
            true,
            FileType::Dir,
        ) {
            Ok(dirs) => {
                for dir in dirs {
                    let folder_path = Path::new(&root_path).join(&dir);
                    if let Ok(uri) = Url::from_file_path(&folder_path) {
                        workspace_folders.push(WorkspaceFolder {
                            uri,
                            name: folder_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }

        if workspace_folders.is_empty() {
            warn!("No workspace folders found. Using root path as workspace.");
            if let Ok(uri) = Url::from_file_path(&root_path) {
                workspace_folders.push(WorkspaceFolder {
                    uri,
                    name: root_path.to_string(),
                });
            }
        }

        Ok(workspace_folders.into_iter().collect())
    }

    pub fn with_initialization_options(mut self, options: serde_json::Value) -> Self {
        self.initialization_options = Some(options);
        self
    }

    pub fn with_setup_workspace_method(mut self, method: String) -> Self {
        self.setup_workspace_method = Some(method);
        self
    }
}
