use lsp_types::{Location, LocationLink};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use utoipa::{IntoParams, ToSchema};

use crate::utils::file_utils::uri_to_relative_path_string;

static GLOBAL_MOUNT_DIR: LazyLock<Arc<RwLock<PathBuf>>> =
    LazyLock::new(|| Arc::new(RwLock::new(PathBuf::from("/mnt/workspace"))));

thread_local! {
    static THREAD_LOCAL_MOUNT_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn get_mount_dir() -> PathBuf {
    THREAD_LOCAL_MOUNT_DIR.with(|local| {
        local
            .borrow()
            .clone()
            .unwrap_or_else(|| GLOBAL_MOUNT_DIR.read().unwrap().clone())
    })
}

pub fn set_thread_local_mount_dir(path: impl AsRef<Path>) {
    THREAD_LOCAL_MOUNT_DIR.with(|local| {
        *local.borrow_mut() = Some(path.as_ref().to_path_buf());
    });
}

pub fn unset_thread_local_mount_dir() {
    THREAD_LOCAL_MOUNT_DIR.with(|local| {
        *local.borrow_mut() = None;
    });
}

pub fn set_global_mount_dir(path: impl AsRef<Path>) {
    let mut global_dir = GLOBAL_MOUNT_DIR.write().unwrap();
    *global_dir = path.as_ref().to_path_buf();
}

/// Response returned when an API error occurs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Description of the error that occurred
    pub error: String,
}

/// Response returned by the health check endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Current status of the service ("ok" or error description)
    pub status: String,
    /// Version of the service
    pub version: String,
    /// Map of supported languages and whether they are currently available
    pub languages: HashMap<SupportedLanguages, bool>,
}

/// Language version string (e.g., "3.4.4" for Ruby, "3.12" for Python)
///
/// This represents the runtime/compiler version of a language, not the container image version.
/// For languages without version detection, this may be None.
#[derive(Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct LanguageVersion(pub String);

impl LanguageVersion {
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the major.minor version (e.g., "3.4" from "3.4.4")
    pub fn minor_version(&self) -> Option<String> {
        let parts: Vec<&str> = self.0.split('.').collect();
        if parts.len() >= 2 {
            Some(format!("{}.{}", parts[0], parts[1]))
        } else {
            None
        }
    }
}

impl Hash for LanguageVersion {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl fmt::Display for LanguageVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for LanguageVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LanguageVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(LanguageVersion(s))
    }
}

/// Language variant for languages that have multiple type system modes
///
/// Some languages support different type-checking modes that require different
/// language servers or configurations. This enum captures those variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum LanguageVariant {
    /// Standard language server without enhanced type checking
    #[default]
    Standard,
    /// Ruby with Sorbet type checker
    Sorbet,
    // Future variants could include:
    // Mypy,     // Python with mypy
    // Pyright,  // Python with pyright
}

impl fmt::Display for LanguageVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageVariant::Standard => write!(f, "standard"),
            LanguageVariant::Sorbet => write!(f, "sorbet"),
        }
    }
}

/// Supported programming languages with optional version information
///
/// This enum represents languages that can be analyzed by the LSP proxy.
/// Languages with version detection (like Ruby) carry version information,
/// while others use a single unversioned container.
#[derive(Debug, Clone, ToSchema)]
pub enum SupportedLanguages {
    Python,
    /// TypeScript and JavaScript are handled by the same langserver
    TypeScriptJavaScript,
    Rust,
    CPP,
    CSharp,
    Java,
    Golang,
    PHP,
    /// Ruby with version and optional Sorbet variant
    Ruby {
        version: LanguageVersion,
        variant: LanguageVariant,
        /// For Sorbet variant: the directory containing sorbet/config (relative to workspace root)
        /// This is used to pass --dir to Sorbet so it can find its config in nested projects
        /// This field is runtime-only and not exposed in the API schema
        #[schema(value_type = Option<String>)]
        sorbet_config_dir: Option<PathBuf>,
    },
}

/// Default Ruby version when none is detected
pub const DEFAULT_RUBY_VERSION: &str = "3.4.7";

impl SupportedLanguages {
    /// Create a Ruby language with version and variant
    pub fn ruby(version: impl Into<String>, variant: LanguageVariant) -> Self {
        SupportedLanguages::Ruby {
            version: LanguageVersion::new(version),
            variant,
            sorbet_config_dir: None,
        }
    }

    /// Create a Ruby language with version, variant, and optional Sorbet config directory
    pub fn ruby_with_config(
        version: impl Into<String>,
        variant: LanguageVariant,
        sorbet_config_dir: Option<PathBuf>,
    ) -> Self {
        SupportedLanguages::Ruby {
            version: LanguageVersion::new(version),
            variant,
            sorbet_config_dir,
        }
    }

    /// Create a Ruby language with default version
    pub fn ruby_default() -> Self {
        Self::ruby(DEFAULT_RUBY_VERSION, LanguageVariant::Standard)
    }

    /// Create a Ruby Sorbet language with default version
    pub fn ruby_sorbet_default() -> Self {
        Self::ruby(DEFAULT_RUBY_VERSION, LanguageVariant::Sorbet)
    }

    /// Convert a Ruby version string and Sorbet flag to the appropriate language
    ///
    /// # Arguments
    /// * `version` - Version string (e.g., "3.4.4", "3.3.6", "3.4")
    /// * `is_sorbet` - Whether this is a Sorbet-enabled Ruby project
    ///
    /// # Returns
    /// Ruby language with appropriate version and variant
    pub fn from_ruby_version(version: &str, is_sorbet: bool) -> Self {
        Self::from_ruby_version_with_config(version, is_sorbet, None)
    }

    /// Convert a Ruby version string, Sorbet flag, and optional config dir to the appropriate language
    ///
    /// # Arguments
    /// * `version` - Version string (e.g., "3.4.4", "3.3.6", "3.4")
    /// * `is_sorbet` - Whether this is a Sorbet-enabled Ruby project
    /// * `sorbet_config_dir` - For Sorbet projects, the directory containing sorbet/config
    ///
    /// # Returns
    /// Ruby language with appropriate version, variant, and config
    pub fn from_ruby_version_with_config(
        version: &str,
        is_sorbet: bool,
        sorbet_config_dir: Option<PathBuf>,
    ) -> Self {
        let normalized = version.trim();
        let variant = if is_sorbet {
            LanguageVariant::Sorbet
        } else {
            LanguageVariant::Standard
        };

        // Normalize partial versions to a stable patch version
        let resolved_version = Self::resolve_ruby_version(normalized);

        SupportedLanguages::Ruby {
            version: LanguageVersion::new(resolved_version),
            variant,
            sorbet_config_dir,
        }
    }

    /// Supported Ruby versions:
    /// - Core versions from original support (3.2.2, 3.2.6, 3.3.5)
    /// - Last 1 year of releases (Nov 2024 - Nov 2025): 3.3.6-3.3.10, 3.4.0-3.4.7
    /// These have dedicated container images with exact version matching.
    const SUPPORTED_RUBY_VERSIONS: &'static [&'static str] = &[
        // Core 3.2.x versions
        "3.2.2", "3.2.6", // 3.3.x series
        "3.3.5", "3.3.6", "3.3.7", "3.3.8", "3.3.9", "3.3.10",
        // 3.4.x series (Dec 2024 - Oct 2025)
        "3.4.0", "3.4.1", "3.4.2", "3.4.3", "3.4.4", "3.4.5", "3.4.6", "3.4.7",
    ];

    /// Resolve a Ruby version string to a supported container version
    ///
    /// Strategy:
    /// 1. Supported versions (last 1 year of releases) are used as-is
    /// 2. Unsupported versions fall back to the default (latest stable)
    /// 3. Ruby 2.x versions are not supported (ruby-lsp gem requires Ruby >= 3.0)
    fn resolve_ruby_version(version: &str) -> String {
        // Check if this is a Ruby 3.x version
        if !version.starts_with("3.") {
            // Ruby 2.x is not supported - fall back to default
            return DEFAULT_RUBY_VERSION.to_string();
        }

        // Check if this is a supported version
        if Self::SUPPORTED_RUBY_VERSIONS.contains(&version) {
            return version.to_string();
        }

        // Unsupported version - fall back to default
        DEFAULT_RUBY_VERSION.to_string()
    }

    /// Check if this language matches a language family
    ///
    /// Used for filtering with ENABLED_LANGUAGES which uses generic names like "ruby"
    /// while actual containers use specific versions.
    pub fn matches_family(&self, family: &SupportedLanguages) -> bool {
        match (self, family) {
            // Exact match for non-versioned languages
            (SupportedLanguages::Python, SupportedLanguages::Python) => true,
            (
                SupportedLanguages::TypeScriptJavaScript,
                SupportedLanguages::TypeScriptJavaScript,
            ) => true,
            (SupportedLanguages::Rust, SupportedLanguages::Rust) => true,
            (SupportedLanguages::CPP, SupportedLanguages::CPP) => true,
            (SupportedLanguages::CSharp, SupportedLanguages::CSharp) => true,
            (SupportedLanguages::Java, SupportedLanguages::Java) => true,
            (SupportedLanguages::Golang, SupportedLanguages::Golang) => true,
            (SupportedLanguages::PHP, SupportedLanguages::PHP) => true,
            // Ruby family matching - any Ruby version matches any other Ruby version
            // regardless of specific version or variant
            (SupportedLanguages::Ruby { .. }, SupportedLanguages::Ruby { .. }) => true,
            _ => false,
        }
    }

    /// Get the language family name (without version/variant)
    pub fn family_name(&self) -> &'static str {
        match self {
            SupportedLanguages::Python => "python",
            SupportedLanguages::TypeScriptJavaScript => "typescript_javascript",
            SupportedLanguages::Rust => "rust",
            SupportedLanguages::CPP => "cpp",
            SupportedLanguages::CSharp => "csharp",
            SupportedLanguages::Java => "java",
            SupportedLanguages::Golang => "golang",
            SupportedLanguages::PHP => "php",
            SupportedLanguages::Ruby { .. } => "ruby",
        }
    }

    /// Check if this is a Ruby language (any version/variant)
    pub fn is_ruby(&self) -> bool {
        matches!(self, SupportedLanguages::Ruby { .. })
    }

    /// Get Ruby version if this is a Ruby language
    pub fn ruby_version(&self) -> Option<&LanguageVersion> {
        match self {
            SupportedLanguages::Ruby { version, .. } => Some(version),
            _ => None,
        }
    }

    /// Get Ruby variant if this is a Ruby language
    pub fn ruby_variant(&self) -> Option<LanguageVariant> {
        match self {
            SupportedLanguages::Ruby { variant, .. } => Some(*variant),
            _ => None,
        }
    }

    /// Get Sorbet config directory if this is a Ruby Sorbet language with a config dir
    pub fn sorbet_config_dir(&self) -> Option<&PathBuf> {
        match self {
            SupportedLanguages::Ruby {
                sorbet_config_dir: Some(dir),
                ..
            } => Some(dir),
            _ => None,
        }
    }
}

// Manual implementations for PartialEq, Eq, and Hash to properly compare Ruby variants
impl PartialEq for SupportedLanguages {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SupportedLanguages::Python, SupportedLanguages::Python) => true,
            (
                SupportedLanguages::TypeScriptJavaScript,
                SupportedLanguages::TypeScriptJavaScript,
            ) => true,
            (SupportedLanguages::Rust, SupportedLanguages::Rust) => true,
            (SupportedLanguages::CPP, SupportedLanguages::CPP) => true,
            (SupportedLanguages::CSharp, SupportedLanguages::CSharp) => true,
            (SupportedLanguages::Java, SupportedLanguages::Java) => true,
            (SupportedLanguages::Golang, SupportedLanguages::Golang) => true,
            (SupportedLanguages::PHP, SupportedLanguages::PHP) => true,
            (
                SupportedLanguages::Ruby {
                    version: v1,
                    variant: var1,
                    ..
                },
                SupportedLanguages::Ruby {
                    version: v2,
                    variant: var2,
                    ..
                },
            ) => v1 == v2 && var1 == var2,
            _ => false,
        }
    }
}

impl Eq for SupportedLanguages {}

impl Hash for SupportedLanguages {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash a discriminant for each variant
        match self {
            SupportedLanguages::Python => 0u8.hash(state),
            SupportedLanguages::TypeScriptJavaScript => 1u8.hash(state),
            SupportedLanguages::Rust => 2u8.hash(state),
            SupportedLanguages::CPP => 3u8.hash(state),
            SupportedLanguages::CSharp => 4u8.hash(state),
            SupportedLanguages::Java => 5u8.hash(state),
            SupportedLanguages::Golang => 6u8.hash(state),
            SupportedLanguages::PHP => 7u8.hash(state),
            SupportedLanguages::Ruby {
                version, variant, ..
            } => {
                8u8.hash(state);
                version.hash(state);
                variant.hash(state);
            }
        }
    }
}

impl fmt::Display for SupportedLanguages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupportedLanguages::Python => write!(f, "python"),
            SupportedLanguages::TypeScriptJavaScript => write!(f, "typescript_javascript"),
            SupportedLanguages::Rust => write!(f, "rust"),
            SupportedLanguages::CPP => write!(f, "cpp"),
            SupportedLanguages::CSharp => write!(f, "csharp"),
            SupportedLanguages::Java => write!(f, "java"),
            SupportedLanguages::Golang => write!(f, "golang"),
            SupportedLanguages::PHP => write!(f, "php"),
            SupportedLanguages::Ruby {
                version, variant, ..
            } => match variant {
                LanguageVariant::Standard => write!(f, "ruby_{}", version.0.replace('.', "_")),
                LanguageVariant::Sorbet => {
                    write!(f, "ruby_sorbet_{}", version.0.replace('.', "_"))
                }
            },
        }
    }
}

// Custom serialization to maintain backward compatibility with existing API
impl Serialize for SupportedLanguages {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize to the same format as before for API compatibility
        let s = match self {
            SupportedLanguages::Python => "python".to_string(),
            SupportedLanguages::TypeScriptJavaScript => "typescript_javascript".to_string(),
            SupportedLanguages::Rust => "rust".to_string(),
            SupportedLanguages::CPP => "cpp".to_string(),
            SupportedLanguages::CSharp => "csharp".to_string(),
            SupportedLanguages::Java => "java".to_string(),
            SupportedLanguages::Golang => "golang".to_string(),
            SupportedLanguages::PHP => "php".to_string(),
            SupportedLanguages::Ruby {
                version, variant, ..
            } => match variant {
                LanguageVariant::Standard => format!("ruby_{}", version.0.replace('.', "_")),
                LanguageVariant::Sorbet => format!("ruby_sorbet_{}", version.0.replace('.', "_")),
            },
        };
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SupportedLanguages {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_str() {
            "python" => Ok(SupportedLanguages::Python),
            "typescript_javascript" => Ok(SupportedLanguages::TypeScriptJavaScript),
            "rust" => Ok(SupportedLanguages::Rust),
            "cpp" => Ok(SupportedLanguages::CPP),
            "csharp" => Ok(SupportedLanguages::CSharp),
            "java" => Ok(SupportedLanguages::Java),
            "golang" => Ok(SupportedLanguages::Golang),
            "php" => Ok(SupportedLanguages::PHP),
            other => {
                // Parse Ruby versions: ruby_X_Y_Z or ruby_sorbet_X_Y_Z
                if let Some(version_part) = other.strip_prefix("ruby_sorbet_") {
                    let version = version_part.replace('_', ".");
                    Ok(SupportedLanguages::Ruby {
                        version: LanguageVersion::new(version),
                        variant: LanguageVariant::Sorbet,
                        sorbet_config_dir: None,
                    })
                } else if let Some(version_part) = other.strip_prefix("ruby_") {
                    let version = version_part.replace('_', ".");
                    Ok(SupportedLanguages::Ruby {
                        version: LanguageVersion::new(version),
                        variant: LanguageVariant::Standard,
                        sorbet_config_dir: None,
                    })
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Unknown language: {}",
                        other
                    )))
                }
            }
        }
    }
}

/// A position within a text document, using 0-based indexing
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Position {
    /// 0-indexed line number.
    #[schema(example = 10)]
    pub line: u32,
    /// 0-indexed character index within the line.
    #[schema(example = 5)]
    pub character: u32,
}

/// A position within a specific file in the workspace
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilePosition {
    /// Path to the file, relative to the workspace root
    #[schema(example = "src/main.py")]
    pub path: String,
    /// Position within the file
    pub position: Position,
}

/// A range within a specific file, defined by start and end positions
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileRange {
    /// The path to the file.
    #[schema(example = "src/main.py")]
    pub path: String,
    /// The range within the file
    pub range: Range,
}

impl FileRange {
    pub fn contains(&self, position: FilePosition) -> bool {
        let pos = &position.position;
        self.path == position.path
            && self.range.start.line <= pos.line
            && self.range.end.line >= pos.line
            && (self.range.start.line != pos.line || self.range.start.character <= pos.character)
            && (self.range.end.line != pos.line || self.range.end.character >= pos.character)
    }
}

impl From<FileRange> for lsp_types::Range {
    fn from(range: FileRange) -> Self {
        lsp_types::Range::new(
            lsp_types::Position::from(range.range.start),
            lsp_types::Position::from(range.range.end),
        )
    }
}

impl From<Position> for lsp_types::Position {
    fn from(position: Position) -> Self {
        lsp_types::Position {
            line: position.line,
            character: position.character,
        }
    }
}

impl From<lsp_types::Position> for Position {
    fn from(position: lsp_types::Position) -> Self {
        Position {
            line: position.line,
            character: position.character,
        }
    }
}

/// A reference to a symbol along with its definition(s) found in the workspace
///
/// e.g. for a reference to `User` in `main.py`:
/// ```python
/// user = User("John", 30)
/// _______^
/// ```
/// This would contain:
/// - The reference location and name ("User" at line 0)
/// - The symbol definition(s) (e.g. "class User" in models.py)
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceWithSymbolDefinitions {
    pub reference: Identifier,
    pub definitions: Vec<Symbol>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct CodeContext {
    pub range: FileRange,
    pub source_code: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Symbol {
    /// The name of the symbol.
    #[schema(example = "User")]
    pub name: String,
    /// The kind of the symbol (e.g., function, class).
    #[schema(example = "class")]
    pub kind: String,

    /// The start position of the symbol's identifier.
    pub identifier_position: FilePosition,

    /// The full range of the symbol.
    pub file_range: FileRange,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Identifier {
    pub name: String,
    pub file_range: FileRange,
    pub kind: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct FindDefinitionRequest {
    pub position: FilePosition,

    /// Whether to include the source code around the symbol's identifier in the response.
    /// Defaults to false.
    /// TODO: Implement this
    #[serde(default)]
    #[schema(example = false)]
    pub include_source_code: bool,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct FindReferencesRequest {
    pub identifier_position: FilePosition,

    /// Whether to include the source code of the symbol in the response.
    /// Defaults to none.
    #[serde(default)]
    #[schema(example = 5)]
    pub include_code_context_lines: Option<u32>,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Request to get all symbols that are referenced from a symbol at the given position, either
/// focusing on function calls, or more permissively finding all references
///
/// The input position must point to a symbol (e.g. function name, class name, variable name).
/// The response will include all symbols that are referenced from that input symbol.
/// For example, if the position points to a function name, the response will include
/// all symbols referenced within that function's implementation.
#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct FindReferencedSymbolsRequest {
    /// Whether to use the more permissive rules to find referenced symbols. This will be not just
    /// code that is executed but also things like type hints and chained indirection.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub full_scan: bool,

    /// The identifier position of the symbol to find references within
    pub identifier_position: FilePosition,
}

/// Request to get the symbols in a file.
#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DefinitionsInFileRequest {
    /// The path to the file to get the symbols for, relative to the root of the workspace.
    #[schema(example = "src/main.py")]
    pub file_path: String,
}

/// Request to get the symbols in the workspace.
#[allow(unused)] // TODO re-implement using textDocument/symbol
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct WorkspaceSymbolsRequest {
    /// The query to search for.
    #[schema(example = "User")]
    pub query: String,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Response to a definition request.
///
/// The definition(s) of the symbol.
/// Points to the start position of the symbol's identifier.
///
/// e.g. for the definition of `User` on line 5 of `src/main.py` with the code:
/// ```text
/// 0: class User:
/// _________^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// 4:
/// 5: user = User("John", 30)
/// __________^
/// ```
/// The definition(s) will be `[{"path": "src/main.py", "line": 0, "character": 6}]`.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FindDefinitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
    pub raw_response: Option<Value>,
    pub definitions: Vec<FilePosition>,
    /// The source code of symbol definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_context: Option<Vec<CodeContext>>,
    /// The identifier that was "clicked-on" to get the definition.
    pub selected_identifier: Identifier,
}

/// Response to a references request.
///
/// Points to the start position of the symbol's identifier.
///
/// e.g. for the references of `User` on line 0 character 6 of `src/main.py` with the code:
/// ```text
/// 0: class User:
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// 4:
/// 5: user = User("John", 30)
/// _________^
/// 6:
/// 7: print(user.name)
/// ```
/// The references will be `[{"path": "src/main.py", "line": 5, "character": 7}]`.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FindReferencesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references
    pub raw_response: Option<Value>,

    pub references: Vec<FilePosition>,

    /// The source code around the references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<CodeContext>>,
    /// The identifier that was "clicked-on" to get the references.
    pub selected_identifier: Identifier,
}

/// Response containing symbols referenced from the requested position
///
/// The symbols are categorized into:
/// - workspace_symbols: References to symbols that were found and have definitions in the workspace
/// - external_symbols: References to symbols from outside the workspace (built-in functions, external libraries)
/// - not_found: References where the symbol definition could not be found
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FindReferencedSymbolsResponse {
    pub workspace_symbols: Vec<ReferenceWithSymbolDefinitions>,
    pub external_symbols: Vec<Identifier>,
    pub not_found: Vec<Identifier>,
}

pub type SymbolResponse = Vec<Symbol>;

impl From<Location> for FilePosition {
    fn from(location: Location) -> Self {
        FilePosition {
            path: uri_to_relative_path_string(&location.uri),
            position: Position {
                line: location.range.start.line,
                character: location.range.start.character,
            },
        }
    }
}

impl From<LocationLink> for FilePosition {
    fn from(link: LocationLink) -> Self {
        FilePosition {
            path: uri_to_relative_path_string(&link.target_uri),
            position: Position {
                line: link.target_range.start.line,
                character: link.target_range.start.character,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FindIdentifierRequest {
    /// The name of the identifier to search for.
    #[schema(example = "User")]
    pub name: String,
    /// The path to the file to search for identifiers.
    #[schema(example = "src/main.py")]
    pub path: String,
    /// The position hint to search for identifiers. If not provided.
    pub position: Option<Position>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FindIdentifierResponse {
    pub identifiers: Vec<Identifier>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Range {
    /// The start position of the range.
    pub start: Position,
    /// The end position of the range.
    pub end: Position,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadSourceCodeRequest {
    /// Path to the file, relative to the workspace root
    #[schema(example = "src/main.py")]
    pub path: String,
    /// Optional range within the file to read
    pub range: Option<Range>,
}

/// Unified JSON-RPC message
///
/// Because multiple message types flow in both directions (requests & notifications
/// from the client, responses & notifications fro the server), it is easier to work
/// with a unified message type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct JsonRpcMessage {
    /// The JSON-RPC version (always "2.0")
    #[schema(example = "2.0")]
    pub jsonrpc: String,

    /// Request ID (required for requests and responses)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_option_value"
    )]
    pub id: Option<Value>,

    /// Method name (required for requests and notifications)
    #[schema(example = "textDocument/hover")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Parameters (optional for requests and notifications)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_option_value"
    )]
    pub params: Option<Value>,

    /// Result (required for responses, unless error is present)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_option_value"
    )]
    pub result: Option<Value>,

    /// Error (required for responses, unless result is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcMessage {
    pub fn new_notification<Id: Serialize, R: Serialize>(
        method: String,
        params: Option<R>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method),
            params: params.map(|params| serde_json::to_value(params).unwrap()),
            result: None,
            error: None,
        }
    }

    pub fn new_request<Id: Serialize, R: Serialize>(
        id: Id,
        method: String,
        params: Option<R>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::to_value(id).unwrap()),
            method: Some(method),
            params: params.map(|params| serde_json::to_value(params).unwrap()),
            result: None,
            error: None,
        }
    }

    pub fn new_result_response<Id: Serialize, R: Serialize>(id: Option<Id>, result: R) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.map_or(Value::Null, |id| serde_json::to_value(id).unwrap())),
            method: None,
            params: None,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    pub fn new_error_response<Id: Serialize>(
        id: Option<Id>,
        code: i32,
        message: impl ToString,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.map_or(Value::Null, |id| serde_json::to_value(id).unwrap())),
            method: None,
            params: None,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JsonRpcError {
    /// Code
    pub code: i32,

    /// Message
    pub message: String,

    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// Custom deserialize function to ensure `Some(Null)` is not reduced to `None`.
/// Fields need to be annotated as follows:
/// ```skip
/// #[serde(
///     default,
///     skip_serializing_if = "Option::is_none",
///     deserialize_with = "deserialize_option_value"
/// )]
/// ```
/// Solution from: <https://github.com/serde-rs/serde/issues/984#issuecomment-314143738>
fn deserialize_option_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_multi_line_range() {
        let range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 12,
                    character: 10,
                },
            },
        };

        // Test positions within the range
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 11,
                    character: 0
                }
            }),
            "middle line should be contained"
        );
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 10,
                    character: 5
                }
            }),
            "start position should be contained"
        );
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 12,
                    character: 10
                }
            }),
            "end position should be contained"
        );
    }

    #[test]
    fn test_contains_multi_line_range_outside_positions() {
        let range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 12,
                    character: 10,
                },
            },
        };

        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 9,
                    character: 0
                }
            }),
            "line before start should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 13,
                    character: 0
                }
            }),
            "line after end should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before start on first line should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 12,
                    character: 11
                }
            }),
            "position after end on last line should not be contained"
        );
    }

    #[test]
    fn test_contains_single_line_range() {
        let single_line_range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 10,
                },
            },
        };

        assert!(
            single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 7
                }
            }),
            "position within single line range should be contained"
        );
        assert!(
            !single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before single line range should not be contained"
        );
        assert!(
            !single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 11
                }
            }),
            "position after single line range should not be contained"
        );
    }

    #[test]
    fn test_contains_zero_width_range() {
        let zero_width_range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 5,
                },
            },
        };

        assert!(
            zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 5
                }
            }),
            "position at zero-width range should be contained"
        );
        assert!(
            !zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before zero-width range should not be contained"
        );
        assert!(
            !zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 6
                }
            }),
            "position after zero-width range should not be contained"
        );
    }
}
