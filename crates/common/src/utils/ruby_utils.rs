use log::debug;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Checks if a Ruby file has Sorbet type annotations.
///
/// Returns true if the file contains a `# typed:` magic comment in the first 10 lines
/// with a type level of "true", "strict", or "strong".
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
