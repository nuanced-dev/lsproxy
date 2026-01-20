pub mod generic;
pub mod golang;
#[allow(unused)]
pub mod python;
pub mod sorbet;

use crate::lsp::client::PostInitializeMessage;
use common::utils::workspace_documents::{
    DidOpenConfiguration, CSHARP_FILE_PATTERNS, C_AND_CPP_FILE_PATTERNS, DEFAULT_EXCLUDE_PATTERNS,
    JAVA_FILE_PATTERNS, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS, RUBY_FILE_PATTERNS,
    RUST_FILE_PATTERNS, TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS,
};
use std::sync::LazyLock;

pub use generic::GenericConfig;
pub use golang::GoplsConfig;
pub use sorbet::SorbetConfig;

pub static PHP_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        PHP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::Lazy,
    )
});

pub static PYTHON_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        PYTHON_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::None,
    )
});

pub static RUBY_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        RUBY_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::None,
    )
});

pub static TYPESCRIPT_AND_JAVASCRIPT_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::Lazy,
    )
});

pub static RUST_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        RUST_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::None,
    )
    .with_initialization_options(serde_json::json!({
        "cargo": {
            "sysroot": serde_json::Value::Null
        }
    }))
    .with_post_initialize_messages(vec![PostInitializeMessage::Request {
        method: "rust-analyzer/reloadWorkspace".to_string(),
        params: None,
    }])
});

pub static JAVA_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::None,
    )
    .with_post_initialize_messages(vec![PostInitializeMessage::ExpectNotification {
        method: "language/status".to_string(),
        params: serde_json::json!({
            "type": "ServiceReady",
            "message": "ServiceReady"
        }),
    }])
});

pub static C_AND_CPP_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        C_AND_CPP_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::Lazy,
    )
    .with_initialization_options(serde_json::json!({
        "clangdFileStatus": true
    }))
});

pub static CSHARP_CONFIG: LazyLock<GenericConfig> = LazyLock::new(|| {
    GenericConfig::new(
        CSHARP_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        DidOpenConfiguration::None,
    )
});
