// Library interface for lsp-wrapper
// Exposes modules for testing while keeping binary entry point in main.rs

// Local modules
pub mod handlers;
pub mod lsp;
pub mod managers;

// Re-export commonly used types
pub use managers::api::ApiManager;

// Test utilities (available in test builds)
#[cfg(test)]
pub mod test_utils;

// AppState struct (shared between lib and bin)
pub struct AppState {
    pub api_manager: ApiManager,
}
