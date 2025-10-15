use crate::{
    api_types::{get_mount_dir, SupportedLanguages},
    lsp::manager::LspManagerError,
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

pub fn search_paths_sequential(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    respect_gitignore: bool,
    file_type: FileType,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    let walk = build_walk(path, exclude_patterns, respect_gitignore);
    for result in walk {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if !include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches_path(path))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                let path = if let Some(path) = file_type.accept(path) {
                    path
                } else {
                    continue;
                };
                paths.push(path.to_path_buf());
            }
            Err(err) => error!("Error: {}", err),
        }
    }

    Ok(paths
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect())
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

                    if !include_patterns.iter().any(|pattern| {
                        glob::Pattern::new(pattern)
                            .map(|p| p.matches_path(path))
                            .unwrap_or(false)
                    }) {
                        return WalkState::Continue;
                    }

                    let path = if let Some(path) = file_type.accept(path) {
                        path
                    } else {
                        return WalkState::Continue;
                    };

                    match paths.lock() {
                        Ok(mut guard) => {
                            guard.push(path.to_path_buf());
                        }
                        Err(poisoned) => {
                            error!("Mutex poisoned, recovering data: {}", poisoned);
                            let mut guard = poisoned.into_inner();
                            guard.push(path.to_path_buf());
                        }
                    }
                }
                Err(err) => error!("Error: {}", err),
            }
            WalkState::Continue
        })
    });

    // Handle Arc unwrap failure
    let mutex = Arc::try_unwrap(paths).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to unwrap Arc: still has outstanding references",
        )
    })?;

    // Handle mutex into_inner failure (poisoned mutex)
    let paths = mutex.into_inner().unwrap_or_else(|poisoned| {
        error!("Mutex was poisoned during parallel search, recovering data");
        poisoned.into_inner()
    });

    Ok(paths
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect())
}

fn build_walk(path: &Path, exclude_patterns: Vec<String>, respect_gitignore: bool) -> ignore::Walk {
    let walk = WalkBuilder::new(path)
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
        .build();
    walk
}

pub fn uri_to_relative_path_string(uri: &Url) -> String {
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

fn has_sorbet_type_annotation(path: &Path) -> bool {
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

pub fn detect_language(file_path: &str) -> Result<SupportedLanguages, LspManagerError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspManagerError::UnsupportedFileType(file_path.to_string()))?;

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
                Ok(SupportedLanguages::RubySorbet)
            } else {
                Ok(SupportedLanguages::Ruby)
            }
        }
        _ => Err(LspManagerError::UnsupportedFileType(file_path.to_string())),
    }
}

pub fn detect_language_string(file_path: &str) -> Result<String, LspManagerError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspManagerError::UnsupportedFileType(file_path.to_string()))?;

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
        _ => Err(LspManagerError::UnsupportedFileType(file_path.to_string())),
    }
}
