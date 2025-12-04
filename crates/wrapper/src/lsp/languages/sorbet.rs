use crate::lsp::client::{LspConfig, CLIENT_CAPABILITES};

use async_trait::async_trait;
use common::utils::workspace_documents::{DidOpenConfiguration, RUBY_FILE_PATTERNS};
use log::{info, warn};
use lsp_types::{InitializeParams, Url, WorkspaceFolder};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tokio::process::Command;

const DEFAULT_RBENV_ROOT: &str = "/opt/rbenv";

pub struct SorbetConfig;

#[async_trait]
impl LspConfig for SorbetConfig {
    #[allow(deprecated)]
    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;

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

    fn get_root_files(&mut self) -> Vec<String> {
        vec![]
    }

    fn include_patterns(&self) -> Vec<String> {
        RUBY_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
    }

    fn exclude_patterns(&self) -> Vec<String> {
        common::utils::workspace_documents::DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect()
    }

    fn did_open_configuration(&self) -> DidOpenConfiguration {
        DidOpenConfiguration::Lazy
    }
}

impl SorbetConfig {
    pub fn new() -> Self {
        Self
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        info!(
            "SorbetConfig::find_workspace_folders called with root_path: {}",
            root_path
        );
        let root = PathBuf::from(&root_path);

        let sorbet_config_path = root.join("sorbet").join("config");
        info!(
            "Looking for sorbet/config at {:?}, exists: {}",
            sorbet_config_path,
            sorbet_config_path.exists()
        );
        if sorbet_config_path.exists() {
            info!("Found sorbet/config at {:?}", sorbet_config_path);

            match fs::read_to_string(&sorbet_config_path) {
                Ok(contents) => {
                    let mut workspace_folders = Vec::new();
                    let mut lines = contents.lines();

                    while let Some(line) = lines.next() {
                        let trimmed = line.trim();

                        if trimmed == "--dir" {
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

impl Default for SorbetConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_sorbet_version(workspace_path: &str) -> Option<String> {
    let lock_path = PathBuf::from(workspace_path).join("Gemfile.lock");
    if let Ok(contents) = fs::read_to_string(&lock_path) {
        if let Some(ver) = parse_sorbet_version_from_lock(&contents) {
            return Some(ver);
        }
    }

    let gemfile_path = PathBuf::from(workspace_path).join("Gemfile");
    if let Ok(contents) = fs::read_to_string(&gemfile_path) {
        if let Some(ver) = parse_sorbet_version_from_gemfile(&contents) {
            return Some(ver);
        }
    }

    None
}

fn parse_sorbet_version_from_lock(lock_contents: &str) -> Option<String> {
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
    for line in gemfile_contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gem") && trimmed.contains("sorbet") {
            let quote = if trimmed.contains('"') { '"' } else { '\'' };
            let parts: Vec<&str> = trimmed.split(quote).collect();
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
