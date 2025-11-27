use crate::manager::Manager;
use actix_web::HttpResponse;
use common::api_types::{
    get_mount_dir, FilePosition, FindReferencedSymbolsRequest, FindReferencedSymbolsResponse,
    Identifier, JsonRpcRequest, JsonRpcResponse, Position, ReferenceWithSymbolDefinitions,
};
use common::utils::file_utils::uri_to_relative_path_string;
use log::{error, info};
use lsp_types::{GotoDefinitionResponse, Position as LspPosition};

/// Find all symbols that are referenced from a given symbol's definition
///
/// The input position must point to a symbol (e.g. function name, class name, variable name).
/// Returns all symbols referenced within that symbol's implementation, categorized into:
/// - Workspace symbols (with their definitions)
/// - External symbols (built-in functions like 'len', 'print' or from external libraries)
/// - Symbols that couldn't be found
///
/// e.g. for a function definition in `main.py`:
/// ```python
/// @log_execution_time     # Reference to decorator
/// def process_user():     # <-- Input position here
///     user = User()       # Reference to User class
///     print("Done")       # Reference to built-in function
/// ```
/// This would return:
/// - Workspace symbols: [
///     log_execution_time (with definition from decorators.py),
///     User (with definition from models.py)
///   ]
/// - External symbols: print (Python built-in)
pub async fn handle(manager: &Manager, request: JsonRpcRequest) -> HttpResponse {
    let req_id = request.id.clone();

    let params = match request.params.clone() {
        Some(p) => p,
        None => {
            error!("Missing parameters for findReferencedSymbols");
            let error = JsonRpcResponse::new_error(req_id, -32602, "Missing params");
            return HttpResponse::BadRequest().json(error);
        }
    };

    let info: FindReferencedSymbolsRequest = match serde_json::from_value(params) {
        Ok(info) => info,
        Err(e) => {
            error!("Invalid parameters for findReferencedSymbols: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32602, format!("Invalid params: {}", e));
            return HttpResponse::BadRequest().json(error);
        }
    };

    info!(
        "Received referenced symbols request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    let referenced_ast_symbols = match manager
        .find_referenced_symbols(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
            info.full_scan,
        )
        .await
    {
        Ok(ast_symbols) => ast_symbols,
        Err(e) => {
            error!("Failed to get referenced symbols: {:?}", e);
            let error = JsonRpcResponse::new_error(
                req_id,
                -32603,
                format!("Failed to get referenced symbols: {}", e),
            );
            return HttpResponse::InternalServerError().json(error);
        }
    };

    let unwrapped_definition_responses: Vec<(Identifier, Vec<FilePosition>)> =
        referenced_ast_symbols
            .into_iter()
            .map(|(ast_grep_result, definition_response)| {
                let definitions = match definition_response {
                    GotoDefinitionResponse::Scalar(location) => vec![FilePosition {
                        path: uri_to_relative_path_string(&location.uri),
                        position: Position {
                            line: location.range.start.line,
                            character: location.range.start.character,
                        },
                    }],
                    GotoDefinitionResponse::Array(locations) => locations
                        .into_iter()
                        .map(|location| FilePosition {
                            path: uri_to_relative_path_string(&location.uri),
                            position: Position {
                                line: location.range.start.line,
                                character: location.range.start.character,
                            },
                        })
                        .collect(),
                    GotoDefinitionResponse::Link(links) => links
                        .into_iter()
                        .map(|link| FilePosition {
                            path: uri_to_relative_path_string(&link.target_uri),
                            position: Position {
                                line: link.target_range.start.line,
                                character: link.target_range.start.character,
                            },
                        })
                        .collect(),
                };
                (Identifier::from(ast_grep_result), definitions)
            })
            .collect();

    // Categorize the definitions
    let mount_dir = get_mount_dir();
    let mut workspace_symbols = Vec::new();
    let mut external_symbols = Vec::new();
    let mut not_found = Vec::new();

    for (identifier, definitions) in unwrapped_definition_responses {
        if definitions.is_empty() {
            not_found.push(identifier);
        } else {
            // Check if any definition is in workspace files
            let has_internal_definition = definitions
                .iter()
                .any(|def| mount_dir.join(&def.path).exists());
            if has_internal_definition {
                let mut symbols_with_definitions = Vec::new();
                for def in definitions
                    .iter()
                    .filter(|def| mount_dir.join(&def.path).exists())
                {
                    let def_position = lsp_types::Position {
                        line: def.position.line,
                        character: def.position.character,
                    };

                    match manager
                        .get_symbol_from_position(&def.path, &def_position)
                        .await
                    {
                        Ok(symbol) => {
                            symbols_with_definitions.push(symbol);
                        }
                        Err(_) => {
                            // Fallback mechanism for position mismatches between LSP operations
                            //
                            // Problem: In some languages (notably TypeScript), textDocument/definition and
                            // documentSymbol may report different character positions for the same symbol.
                            //
                            // Example: TypeScript arrow function properties
                            //   private isWalkable = (point: Point): boolean => { ... }
                            //           ^            ^
                            //           char 12      char 25
                            //
                            // - textDocument/definition returns character 25 (pointing to the arrow =>)
                            // - documentSymbol reports the symbol at character 12 (the identifier name)
                            //
                            // This mismatch causes get_symbol_from_position to fail when using the
                            // textDocument/definition position.
                            //
                            // Solution: Use ast-grep to get all identifiers in the file, find the one
                            // matching by name and line number, then call get_symbol_from_position
                            // using that identifier's position (which aligns with documentSymbol).
                            match manager.get_file_identifiers(&def.path).await {
                                Ok(identifiers) => {
                                    // Find the identifier on the same line as the definition with matching name
                                    if let Some(found_identifier) = identifiers.iter().find(|id| {
                                        id.file_range.range.start.line == def.position.line
                                            && id.name == identifier.name
                                    }) {
                                        // Get the full Symbol using the ast-grep identifier's position
                                        let id_position = lsp_types::Position {
                                            line: found_identifier.file_range.range.start.line,
                                            character: found_identifier
                                                .file_range
                                                .range
                                                .start
                                                .character,
                                        };

                                        if let Ok(symbol) = manager
                                            .get_symbol_from_position(&def.path, &id_position)
                                            .await
                                        {
                                            symbols_with_definitions.push(symbol);
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Both the primary and fallback approaches failed - skip this symbol
                                }
                            }
                        }
                    }
                }
                // Only add to workspace_symbols if we found at least one symbol
                if !symbols_with_definitions.is_empty() {
                    workspace_symbols.push(ReferenceWithSymbolDefinitions {
                        reference: identifier.clone(),
                        definitions: symbols_with_definitions,
                    });
                } else {
                    // If no symbols were found, add to not_found
                    not_found.push(identifier.clone());
                }
            } else {
                external_symbols.push(identifier.clone());
            }
        }
    }

    // Sort workspace_symbols by reference location
    workspace_symbols.sort_by(|a, b| {
        let path_cmp = a
            .reference
            .file_range
            .path
            .cmp(&b.reference.file_range.path);
        if path_cmp.is_eq() {
            a.reference
                .file_range
                .range
                .start
                .line
                .cmp(&b.reference.file_range.range.start.line)
        } else {
            path_cmp
        }
    });

    // Sort external_symbols by location
    external_symbols.sort_by(|a, b| {
        let path_cmp = a.file_range.path.cmp(&b.file_range.path);
        if path_cmp.is_eq() {
            a.file_range
                .range
                .start
                .line
                .cmp(&b.file_range.range.start.line)
        } else {
            path_cmp
        }
    });

    // Sort not_found by location
    not_found.sort_by(|a, b| {
        let path_cmp = a.file_range.path.cmp(&b.file_range.path);
        if path_cmp.is_eq() {
            a.file_range
                .range
                .start
                .line
                .cmp(&b.file_range.range.start.line)
        } else {
            path_cmp
        }
    });

    let response = FindReferencedSymbolsResponse {
        workspace_symbols,
        external_symbols,
        not_found,
    };

    let json_rpc_response = JsonRpcResponse::new_result(req_id, response);

    HttpResponse::Ok().json(json_rpc_response)
}
