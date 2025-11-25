use std::path::PathBuf;

/// Get the workspace root directory (Nuanced LSP project root)
fn workspace_root() -> PathBuf {
    // Get the current directory and walk up to find Cargo.toml
    let mut current_dir = std::env::current_dir().expect("Failed to get current directory");

    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is the workspace root (has [workspace] in Cargo.toml)
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    return current_dir;
                }
            }
        }

        // Move up one directory
        if !current_dir.pop() {
            panic!("Could not find workspace root with [workspace] in Cargo.toml");
        }
    }
}

pub fn python_sample_path() -> String {
    workspace_root()
        .join("sample_project/python")
        .to_string_lossy()
        .to_string()
}

pub fn ruby_sample_path() -> String {
    workspace_root()
        .join("sample_project/ruby")
        .to_string_lossy()
        .to_string()
}

pub fn typescript_sample_path() -> String {
    workspace_root()
        .join("sample_project/typescript")
        .to_string_lossy()
        .to_string()
}

pub fn rust_sample_path() -> String {
    workspace_root()
        .join("sample_project/rust")
        .to_string_lossy()
        .to_string()
}

pub fn csharp_sample_path() -> String {
    workspace_root()
        .join("sample_project/csharp")
        .to_string_lossy()
        .to_string()
}

use common::api_types::set_thread_local_mount_dir;

pub struct TestContext;

impl TestContext {
    pub async fn setup(
        workspace_path: &str,
        _start_servers: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Set the workspace path for the test
        set_thread_local_mount_dir(workspace_path);
        Ok(Self)
    }
}
