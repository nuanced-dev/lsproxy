/// Local LSP client. This module is responsible for communication with the local LSP server.
pub mod client;
pub mod json_rpc;
pub mod languages;
pub mod process;

// Re-export commonly used types
pub use json_rpc::{ExpectedMessageKey, JsonRpcHandler, PendingRequests};
pub use process::ProcessHandler;
