use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
        RUBY_FILE_PATTERNS, RUBY_ROOT_FILES,
    },
};

use async_trait::async_trait;
use lsp_types::InitializeParams;
use notify_debouncer_mini::DebouncedEvent;
use std::{env, error::Error, fs, path::Path, process::Stdio};
use tokio::{process::Command, sync::broadcast::Receiver};

pub const RBENV_ROOT: &str = "/home/user/.rbenv";

pub struct RubySorbetClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for RubySorbetClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }
    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }
    fn get_root_files(&mut self) -> Vec<String> {
        RUBY_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
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
}

pub fn choose_ruby_version(root_path: &str) -> Option<String> {
    if let Some(ver) = detect_project_ruby_version(root_path) {
        log::debug!("Detected Ruby version {}", ver);
        if rbenv_version_installed(&ver) {
            log::debug!("Detected Ruby version installed");
            return Some(ver);
        }

        log::warn!("Detected Ruby version not installed");
        if let Some(global) = rbenv_global() {
            log::warn!("Defaulting to global Ruby version {}", global);
            return Some(global);
        }
    }

    log::warn!("No global Ruby version found; falling back to system Ruby");
    return None;
}

pub fn rbenv_version_installed(ver: &str) -> bool {
    Path::new(RBENV_ROOT).join("versions").join(ver).exists()
}

pub fn rbenv_global() -> Option<String> {
    // ~/.rbenv/version contains the global version if set
    fs::read_to_string(Path::new(RBENV_ROOT).join("version"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn detect_project_ruby_version(root: &str) -> Option<String> {
    // First attempt to use .ruby-version if available.
    let rv = Path::new(root).join(".ruby-version");
    if let Ok(s) = fs::read_to_string(&rv) {
        let v = s.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    // Fallback to parsing the Ruby version from the Gemfile.lock, e.g. "RUBY VERSION\n  ruby 3.1.2p20".
    let gl = Path::new(root).join("Gemfile.lock");
    if let Ok(s) = fs::read_to_string(&gl) {
        let mut in_ruby = false;
        for line in s.lines() {
            let t = line.trim();
            if t == "RUBY VERSION" {
                in_ruby = true;
                continue;
            }
            if in_ruby {
                if let Some(rest) = t.strip_prefix("ruby ") {
                    let ver = rest
                        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                        .next()
                        .unwrap_or("");
                    if !ver.is_empty() {
                        return Some(ver.to_string());
                    }
                }
                break;
            }
        }
    }
    None
}

impl RubySorbetClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let debug_file = std::fs::File::create("/tmp/ruby-sorbet.log")?;

        let mut path = env::var("PATH").unwrap_or_default();
        path = format!("{}/bin:{}/shims:{}", RBENV_ROOT, RBENV_ROOT, path);

        let mut cmd = Command::new("srb");
        cmd.arg("tc")
            .arg("--lsp")
            .arg("--disable-watchman")
            .env("RBENV_ROOT", RBENV_ROOT)
            .env("PATH", path)
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(debug_file);

        if let Some(ver) = choose_ruby_version(root_path) {
            cmd.env("RBENV_VERSION", ver);
        }

        let process = cmd.spawn().map_err(|e| {
            eprintln!("Failed to start ruby-sorbet process: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create RubySorbet ProcessHandler: {}", e))?;

        let json_rpc_handler = JsonRpcHandler::new();
        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            RUBY_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
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
