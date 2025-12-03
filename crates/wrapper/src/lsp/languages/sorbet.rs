use std::fs;
use std::path::PathBuf;

use crate::lsp::client::CLIENT_CAPABILITES;
use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use async_trait::async_trait;
use common::api_types::JsonRpcMessage;
use log::{info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;
use tokio::process::Command;

const DEFAULT_RBENV_ROOT: &str = "/opt/rbenv";

pub struct SorbetClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: common::utils::workspace_documents::WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
    unexpected_notifications_tx: tokio::sync::broadcast::Sender<JsonRpcMessage>,
}

#[async_trait]
impl LspClient for SorbetClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        vec![] // Sorbet doesn't use root files in the new architecture
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

        // Sorbet initialization options for lazy indexing
        let init_options = serde_json::json!({
            "sorbet.lsp.lazyIndexing": true
        });

        Ok(InitializeParams {
            capabilities: CLIENT_CAPABILITES.clone(),
            workspace_folders: Some(workspace_folders),
            root_uri: None,
            initialization_options: Some(init_options),
            ..Default::default()
        })
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        info!(
            "SorbetClient::find_workspace_folders called with root_path: {}",
            root_path
        );
        let root = PathBuf::from(&root_path);

        // 1) Look for sorbet/config file
        let sorbet_config_path = root.join("sorbet").join("config");
        info!(
            "Looking for sorbet/config at {:?}, exists: {}",
            sorbet_config_path,
            sorbet_config_path.exists()
        );
        if sorbet_config_path.exists() {
            info!("Found sorbet/config at {:?}", sorbet_config_path);

            // Parse the config file to find --dir entries
            match fs::read_to_string(&sorbet_config_path) {
                Ok(contents) => {
                    let mut workspace_folders = Vec::new();
                    let mut lines = contents.lines();

                    while let Some(line) = lines.next() {
                        let trimmed = line.trim();

                        // Look for --dir option
                        if trimmed == "--dir" {
                            // The directory path should be on the next line
                            if let Some(dir_line) = lines.next() {
                                let dir_path = dir_line.trim();
                                let full_path = root.join(dir_path);

                                if full_path.exists() {
                                    info!("Adding Sorbet workspace folder: {:?}", full_path);

                                    let uri = Url::from_file_path(&full_path).map_err(|_| {
                                        format!(
                                            "Failed to create URL from path: {}",
                                            full_path.display()
                                        )
                                    })?;

                                    workspace_folders.push(WorkspaceFolder {
                                        uri,
                                        name: full_path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(dir_path)
                                            .to_string(),
                                    });
                                } else {
                                    warn!(
                                        "Sorbet config specifies directory that doesn't exist: {}",
                                        dir_path
                                    );
                                }
                            }
                        }
                    }

                    if !workspace_folders.is_empty() {
                        return Ok(workspace_folders);
                    }

                    warn!("No valid --dir entries found in sorbet/config");
                }
                Err(e) => {
                    warn!("Failed to read sorbet/config: {}", e);
                }
            }
        }

        // 2) Fallback: use the provided root_path
        warn!(
            "No sorbet/config found or no valid directories specified. Falling back to root: {}",
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

impl SorbetClient {
    /// Create a new SorbetClient from the existing GenericLspClient components
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

/// Parse Sorbet version from Gemfile.lock (preferred) or Gemfile
fn parse_sorbet_version(workspace_path: &str) -> Option<String> {
    // Prefer Gemfile.lock
    let lock_path = PathBuf::from(workspace_path).join("Gemfile.lock");
    if let Ok(contents) = fs::read_to_string(&lock_path) {
        if let Some(ver) = parse_sorbet_version_from_lock(&contents) {
            return Some(ver);
        }
    }

    // Fallback to Gemfile
    let gemfile_path = PathBuf::from(workspace_path).join("Gemfile");
    if let Ok(contents) = fs::read_to_string(&gemfile_path) {
        if let Some(ver) = parse_sorbet_version_from_gemfile(&contents) {
            return Some(ver);
        }
    }

    None
}

fn parse_sorbet_version_from_lock(lock_contents: &str) -> Option<String> {
    // Look for lines like "    sorbet (0.5.12414)"
    for line in lock_contents.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("sorbet (") {
            if let Some(end) = rest.find(')') {
                let ver = &rest[..end];
                if !ver.is_empty() {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn parse_sorbet_version_from_gemfile(gemfile_contents: &str) -> Option<String> {
    // Look for lines like: gem "sorbet", "0.5.12414"
    for line in gemfile_contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gem") && trimmed.contains("sorbet") {
            // Support double or single quotes
            let quote = if trimmed.contains('"') { '"' } else { '\'' };
            let parts: Vec<&str> = trimmed.split(quote).collect();
            // parts at odd indices are quoted values
            if parts.len() >= 4 && parts[1].contains("sorbet") {
                let ver = parts[3].trim();
                if !ver.is_empty() {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn rbenv_root() -> String {
    std::env::var("RBENV_ROOT").unwrap_or_else(|_| DEFAULT_RBENV_ROOT.to_string())
}

fn command_with_rbenv_env(cmd: &str) -> Command {
    let root = rbenv_root();
    let mut c = Command::new(cmd);
    let mut path = std::env::var("PATH").unwrap_or_default();
    path = format!("{}/bin:{}/shims:{}", root, root, path);
    c.env("RBENV_ROOT", &root);
    c.env("PATH", path);
    c
}

async fn sorbet_version_installed(version: &str) -> bool {
    let rbenv_bin = format!("{}/bin/rbenv", rbenv_root());
    match command_with_rbenv_env(&rbenv_bin)
        .arg("exec")
        .arg("gem")
        .arg("list")
        .arg("sorbet")
        .arg("-a")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(&format!("sorbet ({version}"))
                || stdout.contains(&format!(", {version}"))
        }
        _ => false,
    }
}

async fn install_sorbet(version: &str) -> Result<(), String> {
    info!("Installing Sorbet version {}", version);

    let rbenv_bin = format!("{}/bin/rbenv", rbenv_root());
    for gem_name in ["sorbet", "sorbet-runtime", "sorbet-static"] {
        let status = command_with_rbenv_env(&rbenv_bin)
            .args([
                "exec",
                "gem",
                "install",
                gem_name,
                "-v",
                version,
                "--no-document",
            ])
            .status()
            .await
            .map_err(|e| format!("Failed to spawn gem install for {}: {e}", gem_name))?;

        if !status.success() {
            return Err(format!(
                "gem install {} -v {} failed with status {}",
                gem_name, version, status
            ));
        }
    }

    // Refresh shims
    let rehash_status = command_with_rbenv_env(&rbenv_bin)
        .arg("rehash")
        .status()
        .await
        .map_err(|e| format!("Failed to spawn rbenv rehash: {e}"))?;

    if !rehash_status.success() {
        return Err(format!("rbenv rehash failed with status {}", rehash_status));
    }

    Ok(())
}

/// Ensure the Sorbet gem version requested by the project is available and return it.
/// Returns Ok(Some(version)) when detected, Ok(None) when unspecified.
pub async fn ensure_sorbet_version(workspace_path: &str) -> Result<Option<String>, String> {
    let Some(desired_version) = parse_sorbet_version(workspace_path) else {
        info!("No Sorbet version specified in Gemfile.lock or Gemfile; using preinstalled version");
        return Ok(None);
    };

    if sorbet_version_installed(&desired_version).await {
        info!("Sorbet version {} already installed", desired_version);
        return Ok(Some(desired_version));
    }

    install_sorbet(&desired_version).await?;
    Ok(Some(desired_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sorbet_from_lock() {
        let lock = r#"
GEM
  specs:
    sorbet (0.5.12414)
    sorbet-runtime (0.5.12414)
    sorbet-static (0.5.12414)
"#;
        assert_eq!(
            parse_sorbet_version_from_lock(lock),
            Some("0.5.12414".to_string())
        );
    }

    #[test]
    fn parses_sorbet_from_gemfile() {
        let gemfile = r#"
source "https://rubygems.org"
gem "sorbet", "0.5.99999"
gem "rails"
"#;
        assert_eq!(
            parse_sorbet_version_from_gemfile(gemfile),
            Some("0.5.99999".to_string())
        );
    }
}
