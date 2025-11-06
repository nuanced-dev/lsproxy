use crate::api_types;

use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&typescript_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "src/PathfinderDisplay.tsx";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("PathfinderDisplay"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 15,
                    character: 13,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 15,
                        character: 0,
                    },
                    end: Position {
                        line: 92,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: String::from("PathfinderDisplayProps"),
            kind: String::from("interface"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 8,
                    character: 10,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 13,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: String::from("astar"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 36,
                    character: 14,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 36,
                        character: 0,
                    },
                    end: Position {
                        line: 36,
                        character: 19,
                    },
                },
            },
        },
        Symbol {
            name: String::from("findPath"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 35,
                    character: 10,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 35,
                        character: 0,
                    },
                    end: Position {
                        line: 41,
                        character: 5,
                    },
                },
            },
        },
        Symbol {
            name: String::from("handleReset"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 65,
                    character: 10,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 65,
                        character: 0,
                    },
                    end: Position {
                        line: 69,
                        character: 5,
                    },
                },
            },
        },
        Symbol {
            name: String::from("newMaze"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 58,
                    character: 14,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 58,
                        character: 0,
                    },
                    end: Position {
                        line: 58,
                        character: 21,
                    },
                },
            },
        },
        Symbol {
            name: String::from("newPath"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 37,
                    character: 14,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 37,
                        character: 0,
                    },
                    end: Position {
                        line: 37,
                        character: 21,
                    },
                },
            },
        },
        Symbol {
            name: String::from("timer"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 45,
                    character: 18,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 45,
                        character: 0,
                    },
                    end: Position {
                        line: 45,
                        character: 23,
                    },
                },
            },
        },
        Symbol {
            name: String::from("toggleCell"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/PathfinderDisplay.tsx"),
                position: Position {
                    line: 55,
                    character: 10,
                },
            },
            file_range: FileRange {
                path: String::from("src/PathfinderDisplay.tsx"),
                range: api_types::Range {
                    start: Position {
                        line: 55,
                        character: 0,
                    },
                    end: Position {
                        line: 63,
                        character: 5,
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
