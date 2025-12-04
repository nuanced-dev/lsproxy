use crate::lsp::client::{LspConfig, CLIENT_CAPABILITES};

use async_trait::async_trait;
use common::utils::file_utils::{search_paths, FileType};
use common::utils::workspace_documents::{
    DEFAULT_EXCLUDE_PATTERNS, PYTHON_FILE_PATTERNS, PYTHON_ROOT_FILES,
};
use log::warn;
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;
use std::path::Path;

pub struct JediConfig;

#[async_trait]
impl LspConfig for JediConfig {
    #[allow(deprecated)]
    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        Ok(InitializeParams {
            capabilities: CLIENT_CAPABILITES.clone(),
            workspace_folders: Some(workspace_folders),
            root_uri: Some(Url::from_file_path(&root_path).unwrap()),
            ..Default::default()
        })
    }

    fn get_root_files(&mut self) -> Vec<String> {
        PYTHON_ROOT_FILES.iter().map(|&s| s.to_string()).collect()
    }

    fn include_patterns(&self) -> Vec<String> {
        PYTHON_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect()
    }

    fn exclude_patterns(&self) -> Vec<String> {
        common::utils::workspace_documents::DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect()
    }

    fn did_open_configuration(&self) -> common::utils::workspace_documents::DidOpenConfiguration {
        common::utils::workspace_documents::DidOpenConfiguration::None
    }
}

impl JediConfig {
    pub fn new() -> Self {
        Self
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
}

impl Default for JediConfig {
    fn default() -> Self {
        Self::new()
    }
}
