use thiserror::Error;

#[derive(Error, Debug)]
pub enum LspError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("LSP error: {0}")]
    LspProtocolError(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Language detection failed: {0}")]
    LanguageDetectionError(String),
}

pub type Result<T> = std::result::Result<T, LspError>;
