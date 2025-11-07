// Library interface for lsp-wrapper
// Exposes modules for testing while keeping binary entry point in main.rs

pub mod api_types;
pub mod ast_grep;
pub mod handlers;
pub mod lsp;
pub mod lsp_process;
pub mod manager;
pub mod utils;

// Re-export commonly used types
pub use manager::Manager;

// Test utilities (available in test builds)
#[cfg(test)]
pub mod test_utils;

// AppState struct (shared between lib and bin)
pub struct AppState {
    pub manager: Manager,
}
