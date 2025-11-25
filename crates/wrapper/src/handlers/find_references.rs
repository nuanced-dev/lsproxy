use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Location, Position as LspPosition};

use crate::handlers::error::IntoHttpResponse;
use crate::handlers::utils;
use crate::manager::{LspManagerError, Manager};
use crate::AppState;
use common::api_types::{
    get_mount_dir, CodeContext, ErrorResponse, FilePosition, FileRange, GetReferencesRequest,
    Position, Range, ReferencesResponse,
};
use common::utils::file_utils::uri_to_relative_path_string;

/// Find all references to a symbol
///
/// The input position should point to the identifier of the symbol you want to get the references for.
///
/// Returns a list of locations where the symbol at the given position is referenced.
///
/// The returned positions point to the start of the reference identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```text
///  0: class User:
///  input____^^^^
///  1:     def __init__(self, name, age):
///  2:         self.name = name
///  3:         self.age = age
///  4:
///  5: user = User("John", 30)
///  output____^
/// ```
#[utoipa::path(
    post,
    path = "/symbol/find-references",
    tag = "symbol",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferencesResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_references(
    data: Data<AppState>,
    info: Json<GetReferencesRequest>,
) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    let file_identifiers = match data
        .manager
        .get_file_identifiers(&info.identifier_position.path)
        .await
    {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };

    let selected_identifier =
        match utils::find_identifier_at_position(file_identifiers, &info.identifier_position).await
        {
            Ok(identifier) => identifier,
            Err(e) => {
                error!("Failed to find references from position: {:?}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!("Failed to find references from position: {}", e),
                });
            }
        };

    let references_result =
        find_and_filter_references(&data.manager, &info.identifier_position).await;
    let code_contexts_result = get_code_contexts(
        &data.manager,
        &references_result,
        info.include_code_context_lines,
    )
    .await;

    match (references_result, code_contexts_result) {
        (Ok(references), Ok(code_contexts)) => {
            let raw_response = if info.include_raw_response {
                match serde_json::to_value(&references) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        error!("Failed to serialize raw response: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let response = ReferencesResponse {
                raw_response,
                references: references
                    .into_iter()
                    .map(|loc| FilePosition {
                        path: uri_to_relative_path_string(&loc.uri),
                        position: Position {
                            line: loc.range.start.line,
                            character: loc.range.start.character,
                        },
                    })
                    .collect(),
                context: code_contexts,
                selected_identifier,
            };
            HttpResponse::Ok().json(response)
        }
        (Err(e), _) => handle_lsp_error(e),
        (_, Err(e)) => {
            error!("Failed to fetch code context: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to fetch code context: {}", e),
            })
        }
    }
}

async fn find_and_filter_references(
    manager: &Manager,
    position: &FilePosition,
) -> Result<Vec<Location>, LspManagerError> {
    let references = manager
        .find_references(
            &position.path,
            LspPosition {
                line: position.position.line,
                character: position.position.character,
            },
        )
        .await?;

    let mount_dir = get_mount_dir();
    let mut filtered_refs: Vec<_> = references
        .into_iter()
        .filter(|reference| {
            let path = uri_to_relative_path_string(&reference.uri);
            mount_dir.join(&path).exists()
        })
        .collect();

    filtered_refs.sort_by(|a, b| {
        let uri_cmp = a.uri.to_string().cmp(&b.uri.to_string());
        if uri_cmp.is_eq() {
            a.range.start.line.cmp(&b.range.start.line)
        } else {
            uri_cmp
        }
    });

    Ok(filtered_refs)
}

async fn get_code_contexts(
    manager: &Manager,
    references_result: &Result<Vec<Location>, LspManagerError>,
    context_lines: Option<u32>,
) -> Result<Option<Vec<CodeContext>>, LspManagerError> {
    match (references_result, context_lines) {
        (Ok(refs), Some(lines)) => fetch_code_context(manager, refs.clone(), lines)
            .await
            .map(Some),
        _ => Ok(None),
    }
}

fn handle_lsp_error(e: LspManagerError) -> HttpResponse {
    e.into_http_response()
}

async fn fetch_code_context(
    manager: &Manager,
    references: Vec<Location>,
    context_lines: u32,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    for reference in references {
        let range = lsp_types::Range {
            start: LspPosition {
                line: reference.range.start.line.saturating_sub(context_lines),
                character: 0,
            },
            end: LspPosition {
                line: reference.range.end.line.saturating_add(context_lines),
                character: 0,
            },
        };
        match manager
            .read_source_code(&uri_to_relative_path_string(&reference.uri), Some(range))
            .await
        {
            Ok(source_code) => {
                code_contexts.push(CodeContext {
                    source_code,
                    range: FileRange {
                        path: uri_to_relative_path_string(&reference.uri),
                        range: Range {
                            start: Position {
                                line: range.start.line,
                                character: 0,
                            },
                            end: Position {
                                line: range.end.line,
                                character: 0,
                            },
                        },
                    },
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(code_contexts)
}
