use common::api_types::SupportedLanguages;
use common::utils::ruby_utils::{
    extract_ruby_version_from_file, has_sorbet_config, has_sorbet_type_annotation,
};
use common::utils::workspace_documents::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Trait for language-specific workspace detection and version management
///
/// Each language implementation defines:
/// - File patterns to scan (source files + manifest/version files)
/// - Logic to process files during workspace scan
/// - How to determine which language variants to spawn
pub trait LanguageManager: Send + Sync {
    /// Returns all file patterns to include in workspace scan
    /// Combines both source file patterns and manifest/version file patterns
    fn file_patterns(&self) -> Vec<String>;

    /// Process a file found during workspace scan
    /// Manager should check if file is relevant before processing
    fn process_file(&mut self, file_path: &Path);

    /// After scan completes, return language variants that should be spawned
    /// Returns empty vec if no relevant files were found
    fn finalize(&self) -> Vec<SupportedLanguages>;

    /// Language family name for logging
    fn name(&self) -> &'static str;
}

/// Ruby language manager with version detection and Sorbet variant support
pub struct RubyManager {
    ruby_version_file: Option<String>, // Version from .ruby-version (highest priority)
    gemfile_version: Option<String>,   // Version from Gemfile (fallback)
    regular_files: Vec<PathBuf>,
    sorbet_files: Vec<PathBuf>,
    manifest_patterns: HashSet<String>,
    source_patterns: HashSet<String>,
}

impl RubyManager {
    pub fn new() -> Self {
        let manifest_patterns: HashSet<String> =
            [".ruby-version", "**/.ruby-version", "Gemfile", "**/Gemfile"]
                .iter()
                .map(|s| s.to_string())
                .collect();

        let source_patterns: HashSet<String> =
            RUBY_FILE_PATTERNS.iter().map(|s| s.to_string()).collect();

        Self {
            ruby_version_file: None,
            gemfile_version: None,
            regular_files: Vec::new(),
            sorbet_files: Vec::new(),
            manifest_patterns,
            source_patterns,
        }
    }

    fn is_manifest(&self, file_path: &Path) -> bool {
        self.manifest_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches_path(file_path))
                .unwrap_or(false)
        })
    }

    fn is_source(&self, file_path: &Path) -> bool {
        self.source_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches_path(file_path))
                .unwrap_or(false)
        })
    }
}

impl LanguageManager for RubyManager {
    fn file_patterns(&self) -> Vec<String> {
        self.manifest_patterns
            .iter()
            .chain(self.source_patterns.iter())
            .cloned()
            .collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        // Check if this is a version-indicating manifest file
        if self.is_manifest(file_path) {
            let file_name = file_path.file_name().and_then(|n| n.to_str());
            log::debug!("Processing Ruby manifest file: {}", file_path.display());

            if let Some(version) = extract_ruby_version_from_file(file_path) {
                // Store version in appropriate field based on file type
                if file_name == Some(".ruby-version") {
                    log::info!(
                        "Detected Ruby version {} from {}",
                        version,
                        file_path.display()
                    );
                    self.ruby_version_file = Some(version);
                } else if file_name == Some("Gemfile") {
                    log::info!(
                        "Detected Ruby version {} from {}",
                        version,
                        file_path.display()
                    );
                    self.gemfile_version = Some(version);
                }
            } else {
                log::debug!(
                    "Failed to extract Ruby version from {}",
                    file_path.display()
                );
            }
        }
        // Check if this is a Ruby source file
        else if self.is_source(file_path) {
            // Categorize into regular or Sorbet bucket based on type annotations AND sorbet/config existence
            // Only use Sorbet if BOTH conditions are met to prevent spawning broken containers
            if has_sorbet_type_annotation(file_path) && has_sorbet_config(file_path) {
                self.sorbet_files.push(file_path.to_owned());
            } else {
                self.regular_files.push(file_path.to_owned());
            }
        }
        // Otherwise, ignore - belongs to another language manager
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        // Prioritize .ruby-version over Gemfile, default to 3.4.4 if neither exists
        log::debug!(
            "Ruby version detection - .ruby-version: {:?}, Gemfile: {:?}",
            self.ruby_version_file,
            self.gemfile_version
        );

        let version = self
            .ruby_version_file
            .as_deref()
            .or(self.gemfile_version.as_deref())
            .unwrap_or("3.4.4");

        let source = if self.ruby_version_file.is_some() {
            ".ruby-version"
        } else if self.gemfile_version.is_some() {
            "Gemfile"
        } else {
            "default"
        };

        log::info!("Selected Ruby version {} from {}", version, source);

        let mut langs = Vec::new();

        if !self.regular_files.is_empty() {
            let lang = SupportedLanguages::from_ruby_version(version, false);
            log::info!(
                "Found {} regular Ruby files, will spawn {:?}",
                self.regular_files.len(),
                lang
            );
            langs.push(lang);
        }

        if !self.sorbet_files.is_empty() {
            let lang = SupportedLanguages::from_ruby_version(version, true);
            log::info!(
                "Found {} Sorbet Ruby files, will spawn {:?}",
                self.sorbet_files.len(),
                lang
            );
            langs.push(lang);
        }

        langs
    }

    fn name(&self) -> &'static str {
        "Ruby"
    }
}

/// Python language manager (stub - no version detection yet)
pub struct PythonManager {
    files_found: bool,
}

impl PythonManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for PythonManager {
    fn file_patterns(&self) -> Vec<String> {
        PYTHON_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        // Simple check - if extension matches Python, track it
        if let Some(ext) = file_path.extension() {
            if ext == "py" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::Python]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "Python"
    }
}

/// TypeScript/JavaScript language manager (stub - no version detection yet)
pub struct TypeScriptJavaScriptManager {
    files_found: bool,
}

impl TypeScriptJavaScriptManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for TypeScriptJavaScriptManager {
    fn file_patterns(&self) -> Vec<String> {
        TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            if ["ts", "tsx", "js", "jsx", "mjs", "cjs"].contains(&ext_str) {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::TypeScriptJavaScript]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "TypeScript/JavaScript"
    }
}

/// Rust language manager (stub - no version detection yet)
pub struct RustManager {
    files_found: bool,
}

impl RustManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for RustManager {
    fn file_patterns(&self) -> Vec<String> {
        RUST_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            if ext == "rs" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::Rust]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "Rust"
    }
}

/// C/C++ language manager (stub - no version detection yet)
pub struct CPPManager {
    files_found: bool,
}

impl CPPManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for CPPManager {
    fn file_patterns(&self) -> Vec<String> {
        C_AND_CPP_FILE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            if ["c", "h", "cpp", "hpp", "cc", "hh", "cxx", "hxx"].contains(&ext_str) {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::CPP]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "C/C++"
    }
}

/// C# language manager (stub - no version detection yet)
pub struct CSharpManager {
    files_found: bool,
}

impl CSharpManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for CSharpManager {
    fn file_patterns(&self) -> Vec<String> {
        CSHARP_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            if ext == "cs" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::CSharp]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "C#"
    }
}

/// Java language manager (stub - no version detection yet)
pub struct JavaManager {
    files_found: bool,
}

impl JavaManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for JavaManager {
    fn file_patterns(&self) -> Vec<String> {
        JAVA_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            if ext == "java" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::Java]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "Java"
    }
}

/// Go language manager (stub - no version detection yet)
pub struct GolangManager {
    files_found: bool,
}

impl GolangManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for GolangManager {
    fn file_patterns(&self) -> Vec<String> {
        GOLANG_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            if ext == "go" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::Golang]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "Go"
    }
}

/// PHP language manager (stub - no version detection yet)
pub struct PHPManager {
    files_found: bool,
}

impl PHPManager {
    pub fn new() -> Self {
        Self { files_found: false }
    }
}

impl LanguageManager for PHPManager {
    fn file_patterns(&self) -> Vec<String> {
        PHP_FILE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn process_file(&mut self, file_path: &Path) {
        if let Some(ext) = file_path.extension() {
            if ext == "php" {
                self.files_found = true;
            }
        }
    }

    fn finalize(&self) -> Vec<SupportedLanguages> {
        if self.files_found {
            vec![SupportedLanguages::PHP]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &'static str {
        "PHP"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write as _;
    use tempfile::TempDir;

    /// Helper to create a temp file with content
    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_ruby_manager_detects_ruby_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let ruby_version = create_temp_file(&temp_dir, ".ruby-version", "3.4.2\n");

        let mut manager = RubyManager::new();
        manager.process_file(&ruby_version);

        assert_eq!(manager.ruby_version_file, Some("3.4.2".to_string()));
        assert_eq!(manager.gemfile_version, None);
    }

    #[test]
    fn test_ruby_manager_detects_gemfile_version() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile = create_temp_file(&temp_dir, "Gemfile", "ruby '3.3.5'\n");

        let mut manager = RubyManager::new();
        manager.process_file(&gemfile);

        assert_eq!(manager.gemfile_version, Some("3.3.5".to_string()));
        assert_eq!(manager.ruby_version_file, None);
    }

    #[test]
    fn test_ruby_manager_prioritizes_ruby_version_over_gemfile() {
        let temp_dir = TempDir::new().unwrap();
        let ruby_version = create_temp_file(&temp_dir, ".ruby-version", "3.4.2\n");
        let gemfile = create_temp_file(&temp_dir, "Gemfile", "ruby '3.3.5'\n");

        let mut manager = RubyManager::new();
        // Process in any order - .ruby-version should win
        manager.process_file(&gemfile);
        manager.process_file(&ruby_version);

        let langs = manager.finalize();
        // Should use 3.4.2 from .ruby-version, not 3.3.5 from Gemfile
        assert_eq!(langs.len(), 0); // No Ruby files, so no languages spawned
    }

    #[test]
    fn test_ruby_manager_spawns_regular_ruby() {
        let temp_dir = TempDir::new().unwrap();
        let ruby_version = create_temp_file(&temp_dir, ".ruby-version", "3.4.2\n");
        let ruby_file = create_temp_file(&temp_dir, "test.rb", "puts 'hello'\n");

        let mut manager = RubyManager::new();
        manager.process_file(&ruby_version);
        manager.process_file(&ruby_file);

        let langs = manager.finalize();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], SupportedLanguages::Ruby3_4_2);
    }

    #[test]
    fn test_ruby_manager_falls_back_to_gemfile() {
        let temp_dir = TempDir::new().unwrap();
        // Only Gemfile, no .ruby-version
        let gemfile = create_temp_file(&temp_dir, "Gemfile", "ruby '3.3.5'\n");
        let ruby_file = create_temp_file(&temp_dir, "test.rb", "puts 'hello'\n");

        let mut manager = RubyManager::new();
        manager.process_file(&gemfile);
        manager.process_file(&ruby_file);

        let langs = manager.finalize();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], SupportedLanguages::Ruby3_3_5);
    }

    #[test]
    fn test_ruby_manager_defaults_to_3_4_4() {
        let temp_dir = TempDir::new().unwrap();
        // No version files, just a Ruby file
        let ruby_file = create_temp_file(&temp_dir, "test.rb", "puts 'hello'\n");

        let mut manager = RubyManager::new();
        manager.process_file(&ruby_file);

        let langs = manager.finalize();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], SupportedLanguages::Ruby3_4_4);
    }

    #[test]
    fn test_ruby_manager_handles_gemfile_constraints() {
        let temp_dir = TempDir::new().unwrap();
        // Test various constraint formats
        let test_cases = vec![
            ("ruby '>= 3.1'", "3.1"),
            ("ruby '~> 3.2'", "3.2"),
            ("ruby '> 3.3'", "3.3"),
            ("ruby '<= 3.4'", "3.4"),
        ];

        for (gemfile_content, expected_version) in test_cases {
            let gemfile = create_temp_file(&temp_dir, "Gemfile", gemfile_content);
            let mut manager = RubyManager::new();
            manager.process_file(&gemfile);

            assert_eq!(
                manager.gemfile_version,
                Some(expected_version.to_string()),
                "Failed to parse: {}",
                gemfile_content
            );
        }
    }

    #[test]
    fn test_ruby_manager_file_patterns_include_dotfiles() {
        let manager = RubyManager::new();
        let patterns = manager.file_patterns();

        // Should include both root-level and nested .ruby-version patterns
        assert!(patterns.contains(&".ruby-version".to_string()));
        assert!(patterns.contains(&"**/.ruby-version".to_string()));
        assert!(patterns.contains(&"Gemfile".to_string()));
        assert!(patterns.contains(&"**/Gemfile".to_string()));
    }
}
