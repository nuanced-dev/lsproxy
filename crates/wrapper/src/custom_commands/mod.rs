use crate::manager::Manager;
use actix_web::HttpResponse;
use common::api_types::JsonRpcRequest;
use log::info;

mod definitions_in_file;
mod find_definition;
mod find_identifier;
mod find_referenced_symbols;
mod find_references;

/// Dispatch custom lsproxy commands to their respective handlers
///
/// Returns Some(response) if the command was handled, None if it should be forwarded to LSP
pub async fn handle_custom_command(
    manager: &Manager,
    request: JsonRpcRequest,
) -> Option<HttpResponse> {
    let method = &request.method;

    let response = match method.as_str() {
        "lsproxy/symbol/findDefinition" => {
            info!("Handling custom command: {}", method);
            find_definition::handle(manager, request).await
        }
        "lsproxy/symbol/findReferences" => {
            info!("Handling custom command: {}", method);
            find_references::handle(manager, request).await
        }
        "lsproxy/symbol/definitionsInFile" => {
            info!("Handling custom command: {}", method);
            definitions_in_file::handle(manager, request).await
        }
        "lsproxy/symbol/findIdentifier" => {
            info!("Handling custom command: {}", method);
            find_identifier::handle(manager, request).await
        }
        "lsproxy/symbol/findReferencedSymbols" => {
            info!("Handling custom command: {}", method);
            find_referenced_symbols::handle(manager, request).await
        }
        _ => {
            return None;
        }
    };

    Some(response)
}
