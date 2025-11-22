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
