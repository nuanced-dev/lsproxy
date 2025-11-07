use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::info;

use crate::api_types::{ErrorResponse, FileSymbolsRequest, Symbol};
use crate::AppState;

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
/// ```text
#[utoipa::path(
    get,
    path = "/symbol/definitions-in-file",
    tag = "symbol",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = Vec<Symbol>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn definitions_in_file(
    data: Data<AppState>,
    info: Query<FileSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received definitions in file request for file: {}",
        info.file_path
    );

    match data
        .manager
        .get_definitions_in_file(&info.file_path)
        .await
    {
        Ok(symbols) => {
            let symbol_response: Vec<Symbol> = symbols
                .into_iter()
                .filter(|s| s.rule_id != "local-variable")
                .map(Symbol::from)
                .collect();
            HttpResponse::Ok().json(symbol_response)
        }
        Err(e) => HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Couldn't get symbols: {}", e),
        }),
    }
}

