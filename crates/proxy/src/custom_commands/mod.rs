use actix_web::HttpResponse;
use common::api_types::JsonRpcRequest;
use log::info;

mod list_files;
mod read_source_code;

/// Dispatch custom lsproxy commands to their respective handlers
///
/// Returns Some(response) if the command was handled, None if it should be forwarded to containers
pub async fn handle_custom_command(request: JsonRpcRequest) -> Option<HttpResponse> {
    let method = &request.method;

    let response = match method.as_str() {
        "lsproxy/workspace/listFiles" => {
            info!("Handling custom command: {}", method);
            list_files::handle(request).await
        }
        "lsproxy/workspace/readSourceCode" => {
            info!("Handling custom command: {}", method);
            read_source_code::handle(request).await
        }
        _ => {
            return None;
        }
    };

    Some(response)
}
