// Common types and utilities shared across all crates
// Layered architecture to avoid circular dependencies:
// 1. error - base error types
// 2. utils - utilities that depend on error
// 3. ast_grep - AST analysis that depends on error
// 4. api_types - API types that depend on all above

pub mod api_types;
pub mod ast_grep;
pub mod error;
pub mod utils;
