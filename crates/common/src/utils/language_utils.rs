use crate::api_types::SupportedLanguages;
use crate::error::LspError;
use std::path::{Path, PathBuf};

use super::ruby_utils::{detect_ruby_version, has_sorbet_config, has_sorbet_type_annotation};
use super::workspace_documents::{
    CPP_EXTENSIONS, CSHARP_EXTENSIONS, C_AND_CPP_EXTENSIONS, C_EXTENSIONS, GOLANG_EXTENSIONS,
    JAVASCRIPTREACT_EXTENSIONS, JAVASCRIPT_EXTENSIONS, JAVA_EXTENSIONS, PHP_EXTENSIONS,
    PYTHON_EXTENSIONS, RUBY_EXTENSIONS, RUST_EXTENSIONS, TYPESCRIPTREACT_EXTENSIONS,
    TYPESCRIPT_AND_JAVASCRIPT_EXTENSIONS, TYPESCRIPT_EXTENSIONS,
};

/// Detect the programming language from a file path.
///
/// Returns a `SupportedLanguages` enum variant based on the file extension.
/// For Ruby files, checks for Sorbet type annotations to determine if it should
/// use RubySorbet or regular Ruby language server.
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

            // Detect Ruby version by walking up to find .ruby-version or Gemfile
            let mut workspace_path = path;
            while let Some(parent) = workspace_path.parent() {
                let has_ruby_version = parent.join(".ruby-version").exists();
                let has_gemfile = parent.join("Gemfile").exists();

                if has_ruby_version || has_gemfile {
                    let version = detect_ruby_version(parent);

                    // Only use Sorbet if BOTH conditions are met:
                    // 1. File has type annotations (# typed: comment)
                    // 2. Workspace has sorbet/config file
                    // This prevents spawning broken Sorbet containers that spin at 100% CPU
                    let is_sorbet = has_sorbet_type_annotation(path) && has_sorbet_config(path);

                    return Ok(SupportedLanguages::from_ruby_version(&version, is_sorbet));
                }

                workspace_path = parent;

                // Stop at root directory
                if parent.parent().is_none() {
                    break;
                }
            }

            // Fallback to default version if no workspace markers found
            Ok(SupportedLanguages::Ruby3_4_4)
        }
        _ => Err(LspError::UnsupportedFileType(file_path.to_string())),
    }
}

/// Detect the programming language from a file path and return it as a string.
///
/// Returns the language name as a lowercase string (e.g., "python", "typescript").
/// This is useful for LSP language IDs.
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
