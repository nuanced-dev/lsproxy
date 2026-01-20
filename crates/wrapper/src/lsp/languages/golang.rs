use crate::lsp::client::{LspConfig, CLIENT_CAPABILITES};

use async_trait::async_trait;
use common::utils::workspace_documents::{DidOpenConfiguration, GOLANG_FILE_PATTERNS};
use log::{info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;
use std::path::{Path, PathBuf};

pub struct GoplsConfig;

#[async_trait]
impl LspConfig for GoplsConfig {
    #[allow(deprecated)]
    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;

        Ok(InitializeParams {
            capabilities: CLIENT_CAPABILITES.clone(),
            workspace_folders: Some(workspace_folders),
            root_uri: None,
            ..Default::default()
        })
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![]
    }

    fn include_patterns(&self) -> Vec<String> {
        GOLANG_FILE_PATTERNS
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

    fn did_open_configuration(&self) -> DidOpenConfiguration {
        DidOpenConfiguration::None
    }
}

impl GoplsConfig {
    pub fn new() -> Self {
        Self
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

impl Default for GoplsConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn nearest_ancestor_with(start: &Path, needle: &str) -> Option<PathBuf> {
    let mut cur = start;

    if cur.is_file() {
        cur = cur.parent()?;
    }

    let mut dir = cur.to_path_buf();
    loop {
        let candidate = dir.join(needle);
        if candidate.exists() {
            return Some(dir);
        }

        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}
