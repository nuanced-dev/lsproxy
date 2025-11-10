use crate::{
    api_types::{get_mount_dir, SupportedLanguages},
    error::LspError,
};
use ignore::WalkBuilder;
use log::{debug, error, warn};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use url::Url;

use super::workspace_documents::{
    CPP_EXTENSIONS, CSHARP_EXTENSIONS, C_AND_CPP_EXTENSIONS, C_EXTENSIONS, GOLANG_EXTENSIONS,
    JAVASCRIPTREACT_EXTENSIONS, JAVASCRIPT_EXTENSIONS, JAVA_EXTENSIONS, PHP_EXTENSIONS,
    PYTHON_EXTENSIONS, RUBY_EXTENSIONS, RUST_EXTENSIONS, TYPESCRIPTREACT_EXTENSIONS,
    TYPESCRIPT_AND_JAVASCRIPT_EXTENSIONS, TYPESCRIPT_EXTENSIONS,
};

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

    let paths = Arc::try_unwrap(paths)
        .unwrap()
        .into_inner()
        .unwrap();

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
    search_paths(root_path, include_patterns, exclude_patterns, true, FileType::Dir)
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

pub fn has_sorbet_type_annotation(path: &Path) -> bool {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().take(10) {
            // Only check first 10 lines for magic comments
            if let Ok(line) = line {
                let trimmed = line.trim();
                if trimmed.starts_with("#") {
                    let comment = trimmed[1..].trim();
                    if comment.starts_with("typed:") {
                        let type_level = comment["typed:".len()..].trim();
                        return matches!(type_level, "true" | "strict" | "strong");
                    }
                }
            }
        }
    }
    false
}

/// Detect Ruby version from workspace
///
/// Looks for .ruby-version file first, then checks Gemfile for ruby version declaration.
/// Returns version string (e.g., "3.4.4") or default "3.4.4" if not found.
pub fn detect_ruby_version(workspace_path: &Path) -> String {
    const DEFAULT_RUBY_VERSION: &str = "3.4.4";

    // First, check for .ruby-version file
    let ruby_version_path = workspace_path.join(".ruby-version");
    if ruby_version_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&ruby_version_path) {
            let version = contents.trim();
            if !version.is_empty() {
                debug!("Found Ruby version {} in .ruby-version", version);
                return version.to_string();
            }
        }
    }

    // Fallback: Check Gemfile for ruby version
    let gemfile_path = workspace_path.join("Gemfile");
    if gemfile_path.exists() {
        if let Ok(file) = File::open(&gemfile_path) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let trimmed = line.trim();
                    // Look for: ruby "3.4.4" or ruby '3.4.4' or ruby "~> 3.4"
                    if trimmed.starts_with("ruby") {
                        // Extract version from quotes
                        if let Some(start) = trimmed.find(|c| c == '"' || c == '\'') {
                            let quote_char = trimmed.chars().nth(start).unwrap();
                            if let Some(end) = trimmed[start + 1..].find(quote_char) {
                                let version_str = &trimmed[start + 1..start + 1 + end];
                                // Handle ~> version specifier (e.g., "~> 3.4")
                                let version = if version_str.starts_with("~>") {
                                    version_str.trim_start_matches("~>").trim()
                                } else {
                                    version_str.trim()
                                };

                                if !version.is_empty() {
                                    debug!("Found Ruby version {} in Gemfile", version);
                                    return version.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    debug!("No Ruby version found, using default {}", DEFAULT_RUBY_VERSION);
    DEFAULT_RUBY_VERSION.to_string()
}

/// Extract Ruby version from a file path if it's a version-indicating file
///
/// Returns Some(version) if the file is .ruby-version or Gemfile and contains version info
/// Returns None otherwise
pub fn extract_ruby_version_from_file(file_path: &Path) -> Option<String> {
    let file_name = file_path.file_name()?.to_str()?;

    match file_name {
        ".ruby-version" => {
            if let Ok(contents) = std::fs::read_to_string(file_path) {
                let version = contents.trim();
                if !version.is_empty() {
                    return Some(version.to_string());
                }
            }
            None
        }
        "Gemfile" => {
            if let Ok(file) = File::open(file_path) {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let trimmed = line.trim();
                        if trimmed.starts_with("ruby") {
                            if let Some(start) = trimmed.find(|c| c == '"' || c == '\'') {
                                let quote_char = trimmed.chars().nth(start)?;
                                if let Some(end) = trimmed[start + 1..].find(quote_char) {
                                    let version_str = &trimmed[start + 1..start + 1 + end];
                                    let version = if version_str.starts_with("~>") {
                                        version_str.trim_start_matches("~>").trim()
                                    } else {
                                        version_str.trim()
                                    };

                                    if !version.is_empty() {
                                        return Some(version.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn detect_language(file_path: &str) -> Result<SupportedLanguages, LspError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspError::UnsupportedFileType(file_path.to_string()))?;

    match extension {
        ext if PYTHON_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Python),
        ext if TYPESCRIPT_AND_JAVASCRIPT_EXTENSIONS.contains(&ext) => {
            Ok(SupportedLanguages::TypeScriptJavaScript)
        }
        ext if RUST_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Rust),
        ext if C_AND_CPP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::CPP),
        ext if CSHARP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::CSharp),
        ext if JAVA_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Java),
        ext if GOLANG_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Golang),
        ext if PHP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::PHP),
        ext if RUBY_EXTENSIONS.contains(&ext) => {
            let path = Path::new(file_path);
            if has_sorbet_type_annotation(path) {
                Ok(SupportedLanguages::RubySorbet3_4_4)
            } else {
                Ok(SupportedLanguages::Ruby3_4_4)
            }
        }
        _ => Err(LspError::UnsupportedFileType(file_path.to_string())),
    }
}

pub fn detect_language_string(file_path: &str) -> Result<String, LspError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspError::UnsupportedFileType(file_path.to_string()))?;

    match extension {
        ext if PYTHON_EXTENSIONS.contains(&ext) => Ok("python".to_string()),
        ext if TYPESCRIPT_EXTENSIONS.contains(&ext) => Ok("typescript".to_string()),
        ext if TYPESCRIPTREACT_EXTENSIONS.contains(&ext) => Ok("typescriptreact".to_string()),
        ext if JAVASCRIPT_EXTENSIONS.contains(&ext) => Ok("javascript".to_string()),
        ext if JAVASCRIPTREACT_EXTENSIONS.contains(&ext) => Ok("javascriptreact".to_string()),
        ext if RUST_EXTENSIONS.contains(&ext) => Ok("rust".to_string()),
        ext if C_EXTENSIONS.contains(&ext) => Ok("c".to_string()),
        ext if CPP_EXTENSIONS.contains(&ext) => Ok("cpp".to_string()),
        ext if CSHARP_EXTENSIONS.contains(&ext) => Ok("csharp".to_string()),
        ext if JAVA_EXTENSIONS.contains(&ext) => Ok("java".to_string()),
        ext if GOLANG_EXTENSIONS.contains(&ext) => Ok("golang".to_string()),
        ext if PHP_EXTENSIONS.contains(&ext) => Ok("php".to_string()),
        ext if RUBY_EXTENSIONS.contains(&ext) => Ok("ruby".to_string()),
        _ => Err(LspError::UnsupportedFileType(file_path.to_string())),
    }
}
