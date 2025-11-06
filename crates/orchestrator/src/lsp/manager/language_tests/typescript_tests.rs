use crate::api_types;

use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&typescript_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "src/node.ts";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("Node"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("src/node.ts"),
                position: Position {
                    line: 0,
                    character: 13,
                },
            },
            file_range: FileRange {
                path: String::from("src/node.ts"),
                range: api_types::Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: String::from("constructor"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("src/node.ts"),
                position: Position {
                    line: 1,
                    character: 4,
                },
            },
            file_range: FileRange {
                path: String::from("src/node.ts"),
                range: api_types::Range {
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 7,
                        character: 8,
                    },
                },
            },
        },
        Symbol {
            name: String::from("f"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("src/node.ts"),
                position: Position {
                    line: 10,
                    character: 4,
                },
            },
            file_range: FileRange {
                path: String::from("src/node.ts"),
                range: api_types::Range {
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 37,
                    },
                },
            },
        },
        Symbol {
            name: String::from("toString"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("src/node.ts"),
                position: Position {
                    line: 13,
                    character: 4,
                },
            },
            file_range: FileRange {
                path: String::from("src/node.ts"),
                range: api_types::Range {
                    start: Position {
                        line: 13,
                        character: 0,
                    },
                    end: Position {
                        line: 13,
                        character: 57,
                    },
                },
            },
        },
    ];
    // sort symbols by name
    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}
