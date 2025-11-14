use crate::api_types::get_mount_dir;
use ignore::WalkBuilder;
use log::{debug, error, warn};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Clone, Copy)]
pub enum FileType {
    Dir,
    File,
}

impl FileType {
    /// Get the effective path that should be added, if possible.
    fn accept(self, path: &Path) -> Option<&Path> {
        match self {
            Self::Dir if path.is_dir() => Some(path),
            Self::Dir if path.is_file() => path.parent(),
            Self::File if path.is_file() => Some(path),
            _ => None,
        }
    }
}

pub fn search_paths(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    respect_gitignore: bool,
    file_type: FileType,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::sync::{Arc, Mutex};

    let paths = Arc::new(Mutex::new(Vec::new()));
    let include_patterns = Arc::new(include_patterns);

    let walker = WalkBuilder::new(path)
        .git_ignore(respect_gitignore)
        .filter_entry(move |entry| {
            let path = entry.path();
            let is_excluded = exclude_patterns.iter().any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches_path(path))
                    .unwrap_or(false)
            });
            !is_excluded
        })
        .build_parallel();

    walker.run(|| {
        let paths = Arc::clone(&paths);
        let include_patterns = Arc::clone(&include_patterns);

        Box::new(move |result| {
            use ignore::WalkState;

            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if include_patterns.iter().any(|pattern| {
                        glob::Pattern::new(pattern)
                            .map(|p| p.matches_path(path))
                            .unwrap_or(false)
                    }) {
                        if let Some(accepted_path) = file_type.accept(path) {
                            if let Ok(mut paths) = paths.lock() {
                                paths.push(accepted_path.to_path_buf());
                            }
                        }
                    }
                }
                Err(err) => error!("Error: {}", err),
            }
            WalkState::Continue
        })
    });

    let paths = Arc::try_unwrap(paths).unwrap().into_inner().unwrap();

    // Deduplicate for Dir type
    if matches!(file_type, FileType::Dir) {
        Ok(paths
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect())
    } else {
        Ok(paths)
    }
}

pub fn search_files(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    respect_gitignore: bool,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    search_paths(
        path,
        include_patterns,
        exclude_patterns,
        respect_gitignore,
        FileType::File,
    )
}

pub fn search_directories(
    root_path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> std::io::Result<Vec<PathBuf>> {
    search_paths(
        root_path,
        include_patterns,
        exclude_patterns,
        true,
        FileType::Dir,
    )
}

pub fn uri_to_relative_path_string(uri: &Url) -> String {
    // Handle case where Sorbet returns relative paths instead of proper file:// URIs
    // If the URI scheme is empty or the path doesn't start with /, it's likely a relative path already
    let uri_str = uri.as_str();

    // Check if this is already a relative path (no scheme, or just a filename)
    if !uri_str.starts_with("file://") && !uri_str.starts_with("/") {
        // Already a relative path, return as-is
        return uri_str.to_string();
    }

    let path = uri.to_file_path().unwrap_or_else(|e| {
        warn!("Failed to convert URI to file path: {:?}", e);
        PathBuf::from(uri.path())
    });

    absolute_path_to_relative_path_string(&path)
}

pub fn absolute_path_to_relative_path_string(path: &PathBuf) -> String {
    let mount_dir = get_mount_dir();
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|e| {
            debug!("Failed to strip prefix from {:?}: {:?}", path, e);
            path.to_string_lossy().into_owned()
        })
}

/// Fixes relative URIs in LSP location responses.
///
/// Some LSP servers (notably Sorbet) return location objects with relative URIs
/// (e.g., "user_service.rb") instead of absolute file:// URIs as required by the
/// LSP specification. This function preprocesses JSON values to convert any
/// relative URIs to absolute file:// URIs based on the workspace root.
///
/// # Arguments
///
/// * `result` - The JSON value containing location data from an LSP response
/// * `workspace_path` - The absolute path to the workspace root directory
///
/// # Returns
///
/// A new JSON value with all relative URIs converted to absolute file:// URIs
pub fn fix_relative_uris(result: serde_json::Value, workspace_path: &str) -> serde_json::Value {
    if let Some(locations_array) = result.as_array() {
        // First pass: check if any URIs need fixing
        let any_needs_fix = locations_array.iter().any(|loc| {
            loc.get("uri")
                .and_then(|uri| uri.as_str())
                .map(|uri_str| !uri_str.starts_with("file://") && !uri_str.starts_with("http"))
                .unwrap_or(false)
        });

        // If no fixes needed, return original result unchanged
        if !any_needs_fix {
            return result;
        }

        // Second pass: fix URIs that need it
        let mut fixed_locations = Vec::with_capacity(locations_array.len());
        for loc in locations_array {
            let needs_fix = loc
                .get("uri")
                .and_then(|uri| uri.as_str())
                .map(|uri_str| !uri_str.starts_with("file://") && !uri_str.starts_with("http"))
                .unwrap_or(false);

            if needs_fix {
                let mut loc_obj = loc.clone();
                if let Some(uri_val) = loc_obj.get_mut("uri") {
                    if let Some(uri_str) = uri_val.as_str() {
                        let abs_path = std::path::PathBuf::from(workspace_path).join(uri_str);
                        if let Ok(abs_uri) = Url::from_file_path(&abs_path) {
                            *uri_val = serde_json::Value::String(abs_uri.to_string());
                        }
                    }
                }
                fixed_locations.push(loc_obj);
            } else {
                fixed_locations.push(loc.clone());
            }
        }
        serde_json::Value::Array(fixed_locations)
    } else {
        result
    }
}
