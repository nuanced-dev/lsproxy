use crate::manager::Manager;
use actix_web::HttpResponse;
use common::api_types::{FileSymbolsRequest, JsonRpcRequest, JsonRpcResponse, Symbol};
use log::{error, info};

/// Get symbols in a specific file (uses ast-grep)
///
/// Returns a list of symbols (functions, classes, variables, etc.) defined in the specified file.
///
/// Only the variabels defined at the file level are included.
///
/// The returned positions point to the start of the symbol's identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```text
/// 0: class User:
/// _________^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// ```
pub async fn handle(manager: &Manager, request: JsonRpcRequest) -> HttpResponse {
    let req_id = request.id.clone();

    let params = match request.params.clone() {
        Some(p) => p,
        None => {
            error!("Missing parameters for definitionsInFile");
            let error = JsonRpcResponse::new_error(req_id, -32602, "Missing params");
            return HttpResponse::BadRequest().json(error);
        }
    };

    let info: FileSymbolsRequest = match serde_json::from_value(params) {
        Ok(info) => info,
        Err(e) => {
            error!("Invalid parameters for definitionsInFile: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32602, format!("Invalid params: {}", e));
            return HttpResponse::BadRequest().json(error);
        }
    };

    info!(
        "Received definitions in file request for file: {}",
        info.file_path
    );

    match manager.get_definitions_in_file(&info.file_path).await {
        Ok(symbols) => {
            let symbol_response: Vec<Symbol> = symbols
                .into_iter()
                .filter(|s| s.rule_id != "local-variable")
                .map(Symbol::from)
                .collect();

            let json_rpc_response = JsonRpcResponse::new_result(req_id, symbol_response);

            HttpResponse::Ok().json(json_rpc_response)
        }
        Err(e) => {
            let error =
                JsonRpcResponse::new_error(req_id, -32603, format!("Couldn't get symbols: {}", e));
            HttpResponse::BadRequest().json(error)
        }
    }
}
