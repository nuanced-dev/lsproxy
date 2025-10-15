use crate::api_types::{get_mount_dir, Identifier, SupportedLanguages, Symbol};
use crate::ast_grep::client::AstGrepClient;
use crate::ast_grep::types::AstGrepMatch;
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    CSharpClient, ClangdClient, GoplsClient, JdtlsClient, JediClient, PhpactorClient, RubyClient,
    RubySorbetClient, RustAnalyzerClient, TypeScriptLanguageClient,
};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::utils::file_utils::{
    absolute_path_to_relative_path_string, detect_language, search_paths, FileType,
};
use crate::utils::workspace_documents::{
    WorkspaceDocuments, CSHARP_FILE_PATTERNS, C_AND_CPP_FILE_PATTERNS, DEFAULT_EXCLUDE_PATTERNS,
    GOLANG_FILE_PATTERNS, JAVA_FILE_PATTERNS, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS,
    RUBY_FILE_PATTERNS, RUBY_SORBET_FILE_PATTERNS, RUST_FILE_PATTERNS,
    TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS,
};
use log::{error, info, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position, Range};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::{channel, Sender};
use tokio::sync::Mutex;

pub struct Manager {
    lsp_clients: HashMap<SupportedLanguages, Arc<Mutex<Box<dyn LspClient>>>>,
    watch_events_sender: Sender<DebouncedEvent>,
    ast_grep: AstGrepClient,
}

impl Manager {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn Error>> {
        let (tx, _) = channel(100);
        let event_sender = tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        let _ = tx.send(event.clone());
                    }
                }
                Err(e) => error!("Debounce error: {:?}", e),
            },
        )
        .expect("Failed to create debouncer");

        // Watch the root path recursively
        debouncer
            .watcher()
            .watch(Path::new(root_path), RecursiveMode::Recursive)
            .expect("Failed to watch path");

        let ast_grep = AstGrepClient {};
        Ok(Self {
            lsp_clients: HashMap::new(),
            watch_events_sender: event_sender,
            ast_grep,
        })
    }

    /// Parses a language string into a SupportedLanguages enum value
    fn parse_language(lang: &str) -> Option<SupportedLanguages> {
        match lang.trim().to_lowercase().as_str() {
            "python" => Some(SupportedLanguages::Python),
            "typescript_javascript" | "typescript" | "javascript" => {
                Some(SupportedLanguages::TypeScriptJavaScript)
            }
            "rust" => Some(SupportedLanguages::Rust),
            "cpp" | "c++" => Some(SupportedLanguages::CPP),
            "csharp" | "c#" => Some(SupportedLanguages::CSharp),
            "java" => Some(SupportedLanguages::Java),
            "golang" | "go" => Some(SupportedLanguages::Golang),
            "php" => Some(SupportedLanguages::PHP),
            "ruby" => Some(SupportedLanguages::Ruby),
            "ruby_sorbet" | "sorbet" => Some(SupportedLanguages::RubySorbet),
            _ => None,
        }
    }

    /// Reads and parses the ENABLED_LANGUAGES environment variable.
    /// Returns None if not set (all languages enabled), or Some(HashSet) with enabled languages.
    fn get_enabled_languages() -> Option<std::collections::HashSet<SupportedLanguages>> {
        std::env::var("ENABLED_LANGUAGES")
            .ok()
            .map(|langs| langs.split(',').filter_map(Self::parse_language).collect())
    }

    /// Detects the languages in the workspace by searching for files that match the language server's file patterns, before LSPs are started.
    /// If ENABLED_LANGUAGES is set, only searches for those languages.
    fn detect_languages_in_workspace(&self, root_path: &str) -> Vec<SupportedLanguages> {
        let enabled_languages = Self::get_enabled_languages();

        let mut lsps = Vec::new();
        for lsp in [
            SupportedLanguages::Python,
            SupportedLanguages::TypeScriptJavaScript,
            SupportedLanguages::Rust,
            SupportedLanguages::CPP,
            SupportedLanguages::CSharp,
            SupportedLanguages::Java,
            SupportedLanguages::Golang,
            SupportedLanguages::PHP,
            SupportedLanguages::Ruby,
            SupportedLanguages::RubySorbet,
        ] {
            // Skip if not in enabled languages (when ENABLED_LANGUAGES is set)
            if let Some(ref enabled) = enabled_languages {
                if !enabled.contains(&lsp) {
                    continue;
                }
            }
            let patterns = match lsp {
                SupportedLanguages::Python => PYTHON_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::TypeScriptJavaScript => TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Rust => {
                    RUST_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::CPP => C_AND_CPP_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::CSharp => CSHARP_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Java => {
                    JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::Golang => GOLANG_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::PHP => {
                    PHP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::Ruby => {
                    RUBY_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::RubySorbet => RUBY_SORBET_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
            };
            if !search_paths(
                Path::new(root_path),
                patterns,
                DEFAULT_EXCLUDE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                true,
                FileType::File,
            )
            .map_err(|e| warn!("Error searching files: {}", e))
            .unwrap_or_default()
            .is_empty()
            {
                lsps.push(lsp);
            }
        }
        info!("Starting LSPs: {:?}", lsps);
        lsps
    }

    pub async fn start_langservers(
        &mut self,
        workspace_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lsps = self.detect_languages_in_workspace(workspace_path);
        for lsp in lsps {
            if self.get_client(lsp).is_some() {
                continue;
            }
            info!("Starting {:?} LSP", lsp);
            let mut client: Box<dyn LspClient> = match lsp {
                SupportedLanguages::Python => Box::new(
                    JediClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::TypeScriptJavaScript => Box::new(
                    TypeScriptLanguageClient::new(
                        workspace_path,
                        self.watch_events_sender.subscribe(),
                    )
                    .await
                    .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Rust => Box::new(
                    RustAnalyzerClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::CPP => Box::new(
                    ClangdClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::CSharp => Box::new(
                    CSharpClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Java => Box::new(
                    JdtlsClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Golang => Box::new(
                    GoplsClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::PHP => Box::new(
                    PhpactorClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Ruby => Box::new(
                    RubyClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::RubySorbet => Box::new(
                    RubySorbetClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
            };
            client
                .initialize(workspace_path.to_string())
                .await
                .map_err(|e| e.to_string())?;
            info!("Setting up workspace");
            client
                .setup_workspace(workspace_path)
                .await
                .map_err(|e| e.to_string())?;
            self.lsp_clients.insert(lsp, Arc::new(Mutex::new(client)));
        }
        Ok(())
    }

    pub async fn definitions_in_file_ast_grep(
        &self,
        file_path: &str,
    ) -> Result<Vec<AstGrepMatch>, LspManagerError> {
        let workspace_files = self.list_files().await?;
        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();

        self.ast_grep
            .get_file_symbols(full_path_str)
            .await
            .map_err(|e| LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e)))
    }

    pub async fn get_symbol_from_position(
        &self,
        file_path: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        match self
            .ast_grep
            .get_symbol_match_from_position(full_path_str, identifier_position)
            .await
        {
            Ok(ast_grep_symbol) => Ok(Symbol::from(ast_grep_symbol)),
            Err(e) => Err(LspManagerError::InternalError(e.to_string())),
        }
    }

    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;

        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        let mut definition = locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
            })?;

        // Sort the locations if there are multiple
        match &mut definition {
            GotoDefinitionResponse::Array(locations) => {
                locations.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.uri);
                    let path_b = uri_to_relative_path_string(&b.uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.range.start.line.cmp(&b.range.start.line))
                        .then(a.range.start.character.cmp(&b.range.start.character))
                });
            }
            GotoDefinitionResponse::Link(links) => {
                links.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.target_uri);
                    let path_b = uri_to_relative_path_string(&b.target_uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.target_range.start.line.cmp(&b.target_range.start.line))
                        .then(
                            a.target_range
                                .start
                                .character
                                .cmp(&b.target_range.start.character),
                        )
                });
            }
            _ => {}
        }
        Ok(definition)
    }

    pub fn get_client(
        &self,
        lsp_type: SupportedLanguages,
    ) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.lsp_clients.get(&lsp_type).cloned()
    }

    pub async fn find_references(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;

        locked_client
            .text_document_reference(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Reference retrieval failed: {}", e))
            })
    }

    pub async fn find_referenced_symbols(
        &self,
        file_path: &str,
        position: Position,
        full_scan: bool,
    ) -> Result<Vec<(AstGrepMatch, GotoDefinitionResponse)>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();

        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;

        // Only Python and TypeScript/JavaScript are currently supported
        match lsp_type {
            SupportedLanguages::Python | SupportedLanguages::TypeScriptJavaScript | SupportedLanguages::CSharp => (),
            _ => return Err(LspManagerError::NotImplemented(
                "Find referenced symbols is only implemented for Python, TypeScript/JavaScript, and C#"
                    .to_string(),
            )),
        }

        // Get the symbol and its references
        let (_, references_to_symbols) = match self
            .ast_grep
            .get_symbol_and_references(full_path_str, &position, full_scan)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Err(LspManagerError::InternalError(format!(
                    "Failed to find referenced symbols, {}",
                    e
                )));
            }
        };

        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        let mut definitions = Vec::new();

        // Get direct definitions for each reference
        for ast_match in references_to_symbols.iter() {
            match locked_client
                .text_document_definition(full_path_str, lsp_types::Position::from(ast_match))
                .await
            {
                Ok(definition) => {
                    definitions.push((ast_match.clone(), definition));
                }
                Err(e) => {
                    // Log the error but continue processing other references
                    log::warn!(
                        "Definition retrieval failed for reference: {}, error: {}",
                        ast_match.meta_variables.single.name.text,
                        e
                    );
                }
            }
        }

        // Only return an error if we couldn't get any definitions at all
        if definitions.is_empty() && !references_to_symbols.is_empty() {
            return Err(LspManagerError::InternalError(
                "Failed to retrieve any definitions for the referenced symbols".to_string(),
            ));
        }

        Ok(definitions)
    }

    pub async fn list_files(&self) -> Result<Vec<String>, LspManagerError> {
        let mut files = Vec::new();
        for client in self.lsp_clients.values() {
            let mut locked_client = client.lock().await;
            files.extend(
                locked_client
                    .get_workspace_documents()
                    .list_files()
                    .await
                    .iter()
                    .filter_map(|f| Some(absolute_path_to_relative_path_string(f)))
                    .collect::<Vec<String>>(),
            );
        }
        files.sort();
        Ok(files)
    }

    pub async fn read_source_code(
        &self,
        file_path: &str,
        range: Option<Range>,
    ) -> Result<String, LspManagerError> {
        let client = self.get_client(detect_language(file_path)?).ok_or(
            LspManagerError::LspClientNotFound(detect_language(file_path)?),
        )?;
        let full_path = get_mount_dir().join(file_path);
        let mut locked_client = client.lock().await;
        locked_client
            .get_workspace_documents()
            .read_text_document(&full_path, range)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Source code retrieval failed: {}", e))
            })
    }

    pub async fn get_file_identifiers(
        &self,
        file_path: &str,
    ) -> Result<Vec<Identifier>, LspManagerError> {
        let full_path = get_mount_dir().join(file_path);
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_identifiers(full_path_str)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;
        Ok(ast_grep_result.into_iter().map(|s| s.into()).collect())
    }
}

#[derive(Debug)]
pub enum LspManagerError {
    FileNotFound(String),
    LspClientNotFound(SupportedLanguages),
    InternalError(String),
    UnsupportedFileType(String),
    NotImplemented(String),
}

impl fmt::Display for LspManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LspManagerError::FileNotFound(path) => {
                write!(f, "File '{}' not found in workspace", path)
            }
            LspManagerError::LspClientNotFound(lang) => {
                write!(f, "LSP client not found for {:?}", lang)
            }
            LspManagerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            LspManagerError::UnsupportedFileType(path) => {
                write!(f, "Unsupported file type: {}", path)
            }
            LspManagerError::NotImplemented(msg) => {
                write!(f, "Not implemented: {}", msg)
            }
        }
    }
}

impl std::error::Error for LspManagerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_parse_language_valid() {
        assert_eq!(
            Manager::parse_language("python"),
            Some(SupportedLanguages::Python)
        );
        assert_eq!(
            Manager::parse_language("PYTHON"),
            Some(SupportedLanguages::Python)
        );
        assert_eq!(
            Manager::parse_language(" python "),
            Some(SupportedLanguages::Python)
        );
    }

    #[test]
    fn test_parse_language_typescript_aliases() {
        assert_eq!(
            Manager::parse_language("typescript"),
            Some(SupportedLanguages::TypeScriptJavaScript)
        );
        assert_eq!(
            Manager::parse_language("javascript"),
            Some(SupportedLanguages::TypeScriptJavaScript)
        );
        assert_eq!(
            Manager::parse_language("typescript_javascript"),
            Some(SupportedLanguages::TypeScriptJavaScript)
        );
    }

    #[test]
    fn test_parse_language_golang_aliases() {
        assert_eq!(
            Manager::parse_language("golang"),
            Some(SupportedLanguages::Golang)
        );
        assert_eq!(
            Manager::parse_language("go"),
            Some(SupportedLanguages::Golang)
        );
    }

    #[test]
    fn test_parse_language_cpp_aliases() {
        assert_eq!(
            Manager::parse_language("cpp"),
            Some(SupportedLanguages::CPP)
        );
        assert_eq!(
            Manager::parse_language("c++"),
            Some(SupportedLanguages::CPP)
        );
    }

    #[test]
    fn test_parse_language_csharp_aliases() {
        assert_eq!(
            Manager::parse_language("csharp"),
            Some(SupportedLanguages::CSharp)
        );
        assert_eq!(
            Manager::parse_language("c#"),
            Some(SupportedLanguages::CSharp)
        );
    }

    #[test]
    fn test_parse_language_ruby_sorbet_aliases() {
        assert_eq!(
            Manager::parse_language("ruby_sorbet"),
            Some(SupportedLanguages::RubySorbet)
        );
        assert_eq!(
            Manager::parse_language("sorbet"),
            Some(SupportedLanguages::RubySorbet)
        );
    }

    #[test]
    fn test_parse_language_invalid() {
        assert_eq!(Manager::parse_language("invalid"), None);
        assert_eq!(Manager::parse_language(""), None);
        assert_eq!(Manager::parse_language("c"), None);
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_not_set() {
        std::env::remove_var("ENABLED_LANGUAGES");
        assert_eq!(Manager::get_enabled_languages(), None);
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_single() {
        std::env::set_var("ENABLED_LANGUAGES", "python");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert!(langs.contains(&SupportedLanguages::Python));
        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_multiple() {
        std::env::set_var("ENABLED_LANGUAGES", "python,rust,typescript");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 3);
        assert!(langs.contains(&SupportedLanguages::Python));
        assert!(langs.contains(&SupportedLanguages::Rust));
        assert!(langs.contains(&SupportedLanguages::TypeScriptJavaScript));
        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_with_spaces() {
        std::env::set_var("ENABLED_LANGUAGES", " python , rust , go ");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 3);
        assert!(langs.contains(&SupportedLanguages::Python));
        assert!(langs.contains(&SupportedLanguages::Rust));
        assert!(langs.contains(&SupportedLanguages::Golang));
        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_with_invalid() {
        std::env::set_var("ENABLED_LANGUAGES", "python,invalid,rust");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 2); // invalid is filtered out
        assert!(langs.contains(&SupportedLanguages::Python));
        assert!(langs.contains(&SupportedLanguages::Rust));
        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_empty_string() {
        std::env::set_var("ENABLED_LANGUAGES", "");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 0); // Empty set
        std::env::remove_var("ENABLED_LANGUAGES");
    }

    #[test]
    #[serial]
    fn test_get_enabled_languages_all_invalid() {
        std::env::set_var("ENABLED_LANGUAGES", "invalid1,invalid2");
        let result = Manager::get_enabled_languages();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 0); // All filtered out
        std::env::remove_var("ENABLED_LANGUAGES");
    }
}
