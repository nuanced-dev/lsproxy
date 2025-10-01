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
/// ```
/// 0: class User:
/// _________^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// ```
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

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::api_types::{FilePosition, FileRange, Position, Range, Symbol};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Query(FileSymbolsRequest {
            file_path: String::from("main.py"),
        });

        let response = definitions_in_file(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let file_symbols_response: Vec<Symbol> = serde_json::from_slice(&bytes).unwrap();

        let expected = vec![
            Symbol {
                name: String::from("plot_path"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 4,
                    },
                },
                file_range: FileRange {
                    path: String::from("main.py"),
                    range: Range {
                        start: Position {
                            line: 5,
                            character: 0,
                        },
                        end: Position {
                            line: 12,
                            character: 14,
                        },
                    },
                },
            },
            Symbol {
                name: String::from("main"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 14,
                        character: 4,
                    },
                },
                file_range: FileRange {
                    path: String::from("main.py"),
                    range: Range {
                        start: Position {
                            line: 14,
                            character: 0,
                        },
                        end: Position {
                            line: 19,
                            character: 28,
                        },
                    },
                },
            },
        ];

        assert_eq!(expected, file_symbols_response);
        Ok(())
    }
}
