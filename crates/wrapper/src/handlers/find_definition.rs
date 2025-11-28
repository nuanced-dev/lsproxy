use crate::handlers::error::IntoHttpResponse;
use crate::manager::{LspManagerError, Manager};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use common::api_types::{CodeContext, FileRange, Position, Range};
use common::utils::file_utils::uri_to_relative_path_string;
use log::{error, info, warn};

use crate::handlers::utils;
use crate::AppState;
use common::api_types::{ErrorResponse, FilePosition};
use common::api_types::{FindDefinitionRequest, FindDefinitionResponse};
use lsp_types::{GotoDefinitionResponse, Location, Position as LspPosition, Range as LspRange};
/// Get the definition of a symbol at a specific position in a file
///
/// Returns the location of the definition for the symbol at the given position.
///
/// The input position should point inside the symbol's identifier, e.g.
///
/// The returned position points to the identifier of the symbol, and the file_path from workspace root
///
/// e.g. for the definition of `User` on line 5 of `src/main.py` with the code:
/// ```text
/// 0: class User:
/// output___^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// 4:
/// 5: user = User("John", 30)
/// input_____^^^^
/// ```
#[utoipa::path(
    post,
    path = "/symbol/find-definition",
    tag = "symbol",
    request_body = FindDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = FindDefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_definition(
    data: Data<AppState>,
    info: Json<FindDefinitionRequest>,
) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.position.line, info.position.position.character
    );

    // Identify the symbol under cursor so we can return a meaningful name
    let file_position = FilePosition {
        path: info.position.path.clone(),
        position: info.position.position.clone(),
    };

    let file_identifiers = match data.manager.get_file_identifiers(&file_position.path).await {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };

    let selected_identifier =
        match utils::find_identifier_at_position(file_identifiers, &file_position).await {
            Ok(identifier) => Some(identifier),
            Err(e) => {
                warn!("Could not resolve identifier at position: {:?}", e);
                None
            }
        };

    // Call LSP directly (no ast-grep for identifier detection)
    let definitions = match data
        .manager
        .find_definition(
            &info.position.path,
            LspPosition {
                line: info.position.position.line,
                character: info.position.position.character,
            },
        )
        .await
    {
        Ok(definitions) => definitions,
        Err(e) => {
            return e.into_http_response();
        }
    };

    let source_code_context = if info.include_source_code {
        match fetch_definition_source_code(&data.manager, &definitions).await {
            Ok(context) => Some(context),
            Err(e) => {
                error!("Failed to fetch definition source code: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    HttpResponse::Ok().json(FindDefinitionResponse {
        raw_response: if info.include_raw_response {
            Some(serde_json::to_value(&definitions).unwrap())
        } else {
            None
        },
        definitions: match &definitions {
            GotoDefinitionResponse::Scalar(location) => vec![location.clone().into()],
            GotoDefinitionResponse::Array(locations) => {
                locations.iter().map(|l| l.clone().into()).collect()
            }
            GotoDefinitionResponse::Link(links) => links.iter().map(|l| l.clone().into()).collect(),
        },
        source_code_context,
        selected_identifier: selected_identifier.unwrap_or_else(|| common::api_types::Identifier {
            name: String::from("(identifier)"),
            kind: None,
            file_range: common::api_types::FileRange {
                path: info.position.path.clone(),
                range: common::api_types::Range {
                    start: info.position.position.clone(),
                    end: info.position.position.clone(),
                },
            },
        }),
    })
}

async fn fetch_definition_source_code(
    manager: &Manager,
    definitions_response: &GotoDefinitionResponse,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    let definitions: &Vec<Location> = match definitions_response {
        GotoDefinitionResponse::Scalar(definition) => &vec![definition.clone()],
        GotoDefinitionResponse::Array(definitions) => definitions,
        GotoDefinitionResponse::Link(links) => &links
            .iter()
            .map(|link| Location::new(link.target_uri.clone(), link.target_range))
            .collect::<Vec<Location>>(),
    };

    for definition in definitions {
        let relative_path = uri_to_relative_path_string(&definition.uri);
        let file_symbols = manager.get_definitions_in_file(&relative_path).await?;
        let symbol = file_symbols.iter().find(|s| {
            s.get_identifier_range().start.line == definition.range.start.line
                && s.get_identifier_range().start.column == definition.range.start.character
        });

        let source_code_context = match symbol {
            Some(ast_grep_match) => CodeContext {
                range: FileRange {
                    path: relative_path,
                    range: Range {
                        start: Position {
                            line: ast_grep_match.get_context_range().start.line,
                            character: ast_grep_match.get_context_range().start.column,
                        },
                        end: Position {
                            line: ast_grep_match.get_context_range().end.line,
                            character: ast_grep_match.get_context_range().end.column,
                        },
                    },
                },
                source_code: ast_grep_match.get_source_code(),
            },
            None => {
                warn!("Symbol not found for definition: {:?}", definition);
                warn!("No exact match in file symbols (likely filtered out). Returning an approximate range instead.");
                let range = LspRange {
                    start: LspPosition {
                        line: definition.range.start.line.saturating_sub(3),
                        character: 0,
                    },
                    end: LspPosition {
                        line: definition.range.end.line.saturating_add(3),
                        character: 0,
                    },
                };
                let source_code = manager
                    .read_source_code(&relative_path, Some(range))
                    .await?;
                CodeContext {
                    range: FileRange {
                        path: relative_path,
                        range: Range {
                            start: Position {
                                line: definition.range.start.line.saturating_sub(3),
                                character: 0,
                            },
                            end: Position {
                                line: definition.range.end.line.saturating_add(3),
                                character: 0,
                            },
                        },
                    },
                    source_code,
                }
            }
        };

        code_contexts.push(source_code_context);
    }
    Ok(code_contexts)
}
