use lsproxy_common::api_types::{set_thread_local_mount_dir, unset_thread_local_mount_dir};
use std::path::PathBuf;

/// Get the workspace root directory (lsproxy project root)
pub fn workspace_root() -> PathBuf {
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

pub fn js_sample_path() -> String {
    workspace_root()
        .join("sample_project/js")
        .to_string_lossy()
        .to_string()
}

pub struct TestContext;

impl TestContext {
    pub async fn setup(file_path: &str, _manager: bool) -> Result<Self, Box<dyn std::error::Error>> {
        set_thread_local_mount_dir(file_path);
        Ok(Self)
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        unset_thread_local_mount_dir();
    }
}
